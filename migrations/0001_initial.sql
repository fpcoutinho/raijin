-- Modelagem híbrida: colunas relacionais para identidade/busca/auditoria/FK;
-- um bloco JSONB por seção temática do laudo (evolui mais rápido que a
-- identidade do laudo, sem exigir ALTER TABLE a cada campo novo).
-- circuits e report_images são tabelas relacionais próprias (1:N real, com
-- FK e cascade delete) — não faz sentido modelar relação 1:N como JSONB.
-- Ver docs/domain-glossary.md, seção "Modelagem do banco: relacional + JSONB
-- por seção", para o raciocínio completo por trás dessa decisão.
--
-- Nomenclatura de campos segue docs/domain-glossary.md — não inventar nomes.
-- gen_random_uuid() é nativo do Postgres 13+, sem extensão necessária.

CREATE TYPE report_status AS ENUM ('draft', 'in_review', 'approved', 'archived');

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- users
-- ============================================================
-- Auth caseira. password_hash e google_id são nullable porque um usuário pode
-- ter só um dos dois métodos; o CHECK garante que tenha pelo menos um.

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    google_id     TEXT UNIQUE,
    avatar_url    TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT users_has_auth_method CHECK (password_hash IS NOT NULL OR google_id IS NOT NULL)
);

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- refresh_tokens
-- ============================================================
-- Sessão longa; o access token vive 15 min e não é persistido em lugar nenhum.
-- O token que vai pro cliente são 32 bytes do CSPRNG do SO e NUNCA é gravado:
-- a coluna guarda só o SHA-256. Dump de banco vazado não vira sessão válida.
-- SHA-256 puro basta — diferente de senha, o token já tem 256 bits de entropia
-- (não há dicionário nem força bruta contra ele) e o lookup precisa ser índice
-- único exato; hash lento com salt por linha exigiria varrer a tabela inteira.
--
-- Rotação a cada uso: /auth/refresh revoga a linha apresentada e emite outra,
-- amarrando as duas por replaced_by. Reapresentar um token já revogado há mais
-- de alguns segundos é sinal de token roubado — a aplicação revoga a cadeia
-- inteira daquele usuário (ver http::auth::routes::refresh). Dentro de uma
-- janela curta, é tratado como replay legítimo de múltiplas abas.

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  BYTEA NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    replaced_by UUID REFERENCES refresh_tokens(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Revogação em massa (logout total, takeover do Google, reuso detectado).
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens (user_id);

-- Varredura da coleta de lixo de sessões expiradas — mesmo padrão do índice
-- parcial de report_images pendentes.
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;

CREATE TRIGGER trg_refresh_tokens_updated_at
    BEFORE UPDATE ON refresh_tokens
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- reports
-- ============================================================
-- location_code segue o padrão BLOCO-SALA (ex.: CCHLA-102); validação de
-- formato fica na aplicação (regex), não em CHECK constraint, para poder
-- evoluir sem migration. O prefixo antes do "-" é o "bloco", usado no
-- auto-preenchimento de inspection_planning ao criar um laudo no mesmo bloco.

CREATE TABLE reports (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    author_id               UUID NOT NULL REFERENCES users(id),
    location_code           TEXT NOT NULL,
    inspected_at            TIMESTAMPTZ NOT NULL,
    ambient_temperature_c   INTEGER,
    weather_conditions      TEXT,
    responsible_parties     TEXT[] NOT NULL DEFAULT '{}',
    status                  report_status NOT NULL DEFAULT 'draft',

    -- Seções do laudo. Ver docs/domain-glossary.md para o schema de cada uma.
    -- NULL = seção ainda não preenchida. Não usar '{}' como default: as quatro
    -- primeiras têm todos os campos obrigatórios, então objeto vazio não é um
    -- valor válido da seção — seria dado inventado esperando pra falhar.
    inspection_planning     JSONB,
    external_influences     JSONB,
    qualitative_assessment  JSONB,
    quantitative_assessment JSONB,
    -- document_content é texto livre gerado; '{}' é estado inicial legítimo.
    document_content        JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_reports_location_code ON reports (location_code);
CREATE INDEX idx_reports_author_id ON reports (author_id);
CREATE INDEX idx_reports_status ON reports (status);

CREATE TRIGGER trg_reports_updated_at
    BEFORE UPDATE ON reports
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- circuits
-- ============================================================
-- Uma linha por circuito do quadro de distribuição.

CREATE TABLE circuits (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id     UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    circuit_model TEXT,
    phase         TEXT,
    breaker       TEXT,
    description   TEXT,
    conductor     TEXT,
    current       NUMERIC,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_circuits_report_id ON circuits (report_id);

CREATE TRIGGER trg_circuits_updated_at
    BEFORE UPDATE ON circuits
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- report_images
-- ============================================================
-- storage_path referencia o objeto no bucket (upload feito via URL
-- pré-assinada, gerada pelo backend — ver docs do Step 2). finding_category
-- é lista aberta (as 5 categorias de docs/findings-taxonomy.md), não um enum
-- de banco: a taxonomia pode crescer sem exigir migration.
--
-- Upload em duas etapas: o backend cria a linha em 'pending' e grava
-- storage_path na hora de assinar a URL de escrita, ANTES do upload
-- acontecer. Na confirmação o frontend manda só o image_id — nunca o path —
-- porque o servidor não confia em referência de objeto vinda do cliente; ele
-- confirma contra o objeto real do bucket (HEAD) e só então marca 'uploaded',
-- gravando content_type/size_bytes lidos de lá, não do que o cliente alega
-- ter enviado. Linha 'pending' velha = upload abandonado, lixo coletável.
--
-- caption/position: legenda e ordem no apêndice fotográfico (ver
-- docs/findings-taxonomy.md, "Padrão de diagramação"). O legado empilhava
-- fotos sem legenda nem agrupamento — não tinha nenhum dos dois campos.
--
-- No legado a imagem era um CloudinaryField baixado por urllib a cada
-- exportação, sem cache e sem metadado nenhum. Aqui o bytes nunca passa pelo
-- backend, e o que fica no banco é só a referência + metadado verificado.

CREATE TYPE image_upload_status AS ENUM ('pending', 'uploaded');

CREATE TABLE report_images (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id        UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    storage_path     TEXT NOT NULL UNIQUE,
    finding_category TEXT,
    upload_status    image_upload_status NOT NULL DEFAULT 'pending',
    content_type     TEXT,
    size_bytes       BIGINT,
    uploaded_at      TIMESTAMPTZ,
    caption          TEXT,
    position         INTEGER NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT report_images_uploaded_has_metadata CHECK (
        upload_status = 'pending'
        OR (content_type IS NOT NULL AND size_bytes IS NOT NULL AND uploaded_at IS NOT NULL)
    )
);

-- Listagem do laudo é sempre "imagens confirmadas, na ordem do apêndice".
CREATE INDEX idx_report_images_report_id ON report_images (report_id, upload_status, position);

-- Varredura da coleta de lixo de uploads abandonados.
CREATE INDEX idx_report_images_pending ON report_images (created_at)
    WHERE upload_status = 'pending';

CREATE TRIGGER trg_report_images_updated_at
    BEFORE UPDATE ON report_images
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
