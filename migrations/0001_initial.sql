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
    inspection_planning     JSONB NOT NULL DEFAULT '{}'::jsonb,
    external_influences     JSONB NOT NULL DEFAULT '{}'::jsonb,
    qualitative_assessment  JSONB NOT NULL DEFAULT '{}'::jsonb,
    quantitative_assessment JSONB NOT NULL DEFAULT '{}'::jsonb,
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
-- Uma linha por circuito do quadro de distribuição. Sem limite de linhas
-- (o legado truncava em 13 por limitação do template Word — não replicar).
-- circuit_id é o rótulo "Circuito" no domain-glossary.md; nome de coluna
-- diferente de "circuito" pra não colidir com o nome da entidade.

CREATE TABLE circuits (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id   UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    circuit_id  TEXT,
    phase       TEXT,
    breaker     TEXT,
    description TEXT,
    conductor   TEXT,
    current     NUMERIC,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
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

CREATE TABLE report_images (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id        UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    storage_path     TEXT NOT NULL,
    finding_category TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_report_images_report_id ON report_images (report_id);

CREATE TRIGGER trg_report_images_updated_at
    BEFORE UPDATE ON report_images
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
