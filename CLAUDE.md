# CLAUDE.md

Backend do sistema de Laudos de Engenharia Elétrica. API REST em Rust, banco Postgres. Substitui o monolito Django legado (repositório `gerador`, congelado, referência apenas — **não releia esse repositório**, todo conhecimento de domínio necessário já foi extraído para `docs/` abaixo).

## Repositórios do projeto

| Repositório | Conteúdo |
|---|---|
| `gerador` | Monolito Django legado. Congelado — só consulta se `docs/` daqui não bastar. |
| `raijin` (este) | Backend Rust/Axum + schema e migrations do banco. |
| `itui` | Frontend React/Vite + Design System Sanhauá. Contrato entre os dois é a API REST, especificado em [`docs/api-contract.md`](docs/api-contract.md) — rota nova entra lá antes de existir em código. |

## Stack

- **Linguagem/framework**: Rust + Axum.
- **Banco**: Postgres — **Neon** em produção, dev local via Docker (ver `docker-compose.yml`). O schema não usa recurso exclusivo de nenhum provedor, então trocar de Postgres gerenciado não exige migration.
- **Acesso a dados**: SQLx (queries verificadas em compile-time contra o banco).
- **Runtime**: serverless na **AWS Lambda** (`cargo-lambda` + `lambda_http`), `axum::Router` exposto pelo adaptador. **Dual runtime**: `main.rs` detecta `AWS_LAMBDA_RUNTIME_API` em tempo de execução e ramifica entre `lambda_http::run` e `axum::serve` — nenhum handler/middleware/extractor muda entre os dois. Dev local do dia a dia por `cargo run` (contra o Postgres do Docker); `cargo lambda watch` só quando o teste depende do runtime Lambda de verdade (ver "Verificação" abaixo).
- **Auth**: caseira — `argon2` para senha, `jsonwebtoken` tanto para os nossos JWT (HS256) quanto para validar o ID Token da Google (RS256, contra o JWKS público). **Sem a crate `oauth2` e sem authorization-code flow** — o `itui` usa Google Identity Services e manda o ID Token; o backend só verifica assinatura. Dito no negativo de propósito: é decisão fechada, não lacuna.
- **Storage de imagens**: upload direto do frontend via URL pré-assinada (o backend gera a URL, não faz proxy de bytes). Provedor: **Cloudflare R2** em produção, **MinIO** localmente em dev — protocolo S3-compatible, único ponto do código específico de provedor isolado atrás do trait `storage::ObjectStorage`. Bucket **privado** em ambos os ambientes; leitura também por URL assinada (nunca habilitar domínio público `r2.dev`/custom domain no bucket) — um laudo fotografa vulnerabilidade física real de uma edificação identificada por `location_code`, então bucket público é vazamento de mapa de vulnerabilidade.
- **IA**: proxy para **Groq** (Llama 3.3 70B / GPT-OSS 120B — free tier sem cartão, sem custo por token) via SSE, streaming direto pro frontend. Isolado atrás do trait `llm::TextGenerator`, mesmo padrão do storage: trocar Groq → Outra LLM (plano B se o limite de tokens/dia apertar) vira nova implementação do trait, sem tocar em `http::` nem no `itui`. **Prompt nunca inclui `location_code`** — só categoria do achado e descrição; a mesma razão do bucket privado.

## Convenções de código

- Tudo em inglês: funções, variáveis, tipos, colunas, rotas. `snake_case` para funções/variáveis/campos, `PascalCase` para tipos/structs/enums, `SCREAMING_SNAKE_CASE` para constantes.
- Português só em conteúdo voltado ao usuário final (mensagens de erro exibidas, conteúdo do laudo) — nunca em código.
- **Não invente nomes de campo.** Toda a nomenclatura do domínio (campos do laudo, seções, enums) vem de [`docs/domain-glossary.md`](docs/domain-glossary.md). Se faltar algo lá, adicione antes de usar em código.
- **Sem doc comment (`//!`/`///`) de arquivo ou módulo explicando arquitetura, papel do módulo ou paralelo com outra stack.** Isso já está no README, neste CLAUDE.md e em `docs/` — repetir no topo do arquivo é manutenção duplicada (o comentário fica velho e ninguém percebe). Comentário só perto do código que ele explica, e só quando o porquê não é óbvio (regra não-óbvia, workaround, invariante escondida) — nunca resumindo o que o código já deixa claro por si.
- **Comentário de código é curto — uma ideia, uma frase.** Nada de empilhar em um único comentário a razão, a alternativa descartada, a consequência e a nota de manutenção futura. Se um comentário de função precisa de mais de ~3 linhas pra dizer "por que" (não "o quê"), é sinal de dividir: a frase principal fica no comentário, o resto (trade-off, alternativa considerada) só entra se for realmente decidir algo pra quem for mexer depois — e mesmo assim curto.
- **NÃO dispare subagentes para buscar na web sintaxe ou APIs de Rust/Axum/SQLx/Tokio**. Escreva o código usando seu conhecimento embutido. O compilador do Rust (cargo check) é a fonte da verdade e o validador final, não a internet. Se houver um erro de tipo ou trait bound, o cargo check apontará a linha exata; corrija o erro localmente em vez de pesquisar online.
- **Pesquisa na web é exceção, não regra**. Só recorra a subagentes (e se o fizer, use esforço baixo/mínimo) se for para integrar uma dependência totalmente nova e complexa que não está nesta stack, ou para resolver um erro de runtime obscuro que o compilador não explica. Para sintaxe padrão do Rust e uso das crates já definidas, é proibido.

## Documentação de domínio

Extraída do legado no "Step 0" da migração — leia antes de mexer no schema ou nas regras de negócio:

- [`docs/domain-glossary.md`](docs/domain-glossary.md) — mapa canônico dos ~90 campos do laudo (nome legado → nome novo, rótulo pt-BR, tipo, regras por seção), decisões de modelagem já fechadas.
- [`docs/nbr-5410-choices.json`](docs/nbr-5410-choices.json) — listas de opções normativas (NBR 5410) por campo, com cláusula da norma. Fonte única — não hardcode essas listas em Rust nem crie tabela de referência no banco pra isso.
- [`docs/nbr-5410-tests.md`](docs/nbr-5410-tests.md) — os 6 ensaios da avaliação quantitativa: procedimento, critério numérico de aceitação, cláusula normativa. A regra de espaço-reserva do quadro de distribuição (a única conta real do domínio) está aqui.
- [`docs/findings-taxonomy.md`](docs/findings-taxonomy.md) — taxonomia de 5 categorias de não conformidade + parecer de referência. Usado para categorizar imagens e como few-shot do prompt da Groq.

## Decisões de arquitetura já fechadas (não reabrir sem motivo novo)

- **Modelagem do banco**: híbrida. Colunas relacionais para identidade/busca/auditoria/FK; um JSONB por seção temática do laudo (`inspection_planning`, `external_influences`, `qualitative_assessment`, `quantitative_assessment`, `document_content`). `circuits` e `report_images` são tabelas relacionais próprias (1:N), não JSONB.
- **Sem limite de circuitos.** O legado truncava em 13 (limitação do template Word). A nova stack itera livremente.
- **Avaliação qualitativa é ternária** (Sim/Não/Parcialmente), não booleana. Ensaios (avaliação quantitativa Parte II) são binários (Sim/Não).
- **Medições numéricas usam `numeric` no Postgres / `rust_decimal::Decimal` no Rust**, não `float`/`f64` — evita perda de precisão em valor de medição.
- **`spare_circuit_capacity`** deve ser acompanhado do cálculo real do espaço-reserva (NBR 5410 6.5.4.7), não só guardar a faixa escolhida como o legado fazia.
- **Auth**: tabela `users` própria (não delegar pra serviços terceiros de auth). Login Google e e-mail/senha convergem pro mesmo usuário quando o e-mail bate.
- **`.env` nunca committado.** Segredos (JWT secret, client secret do Google, `DATABASE_URL`) só via variável de ambiente — o legado tinha credenciais em texto puro no repo, não repetir.
- **Upload de imagem em duas etapas**: o backend cria a linha `report_images` com `upload_status = 'pending'` e o `storage_path` **antes** do upload acontecer, na hora de assinar a URL de escrita. A confirmação do frontend manda só o `image_id` — nunca o `storage_path` — porque o servidor não confia em referência de objeto vinda do cliente; ele confirma contra o objeto real do bucket (`HEAD`) e só então marca `uploaded`, gravando `content_type`/`size_bytes` lidos de lá, não do que o cliente alega ter enviado.
- **Slugs de `finding_category`** vivem em [`docs/findings-taxonomy.md`](docs/findings-taxonomy.md) (seção "Identificadores canônicos"), espelhados em `domain::FINDING_CATEGORIES`. Lista aberta validada na aplicação — não é enum de banco, pra taxonomia poder crescer sem migration.
- **SQL só vive em `queries.rs`**, um por feature dentro de `http::`, mesmo em rotas pequenas onde caberia inline no handler. Regra greppável e verificável, não "extrai quando doer" — decisão deliberada mesmo sabendo que a implementação de referência do SQLx ([`launchbadge/realworld-axum-sqlx`](https://github.com/launchbadge/realworld-axum-sqlx)) faz o oposto.
- **Deploy: serverless na AWS Lambda**, via `cargo-lambda` + `lambda_http`. O `axum::Router` em si é agnóstico a isso — só o entrypoint em `main.rs` sabe se está atrás de `axum::serve` (TCP) ou do adaptador Lambda. Consequências em outras decisões desta lista:
  - **Sem `tokio::spawn` de longa duração.** Tarefa de fundo (ex.: limpeza de sessão expirada) é endpoint HTTP protegido, disparado por agendamento externo (AWS EventBridge Scheduler), não loop em processo — Lambda não garante processo vivo entre invocações.
  - **`DATABASE_URL` de produção aponta pro endpoint com pooling do Neon** (PgBouncer), não a conexão direta — instâncias concorrentes de Lambda multiplicam conexões rápido o bastante pra estourar o limite do Postgres gerenciado sem isso.
  - **Rate limiting (`tower_governor`) é por instância**, não global — cada instância fria de Lambda tem seu próprio balde de contagem. Aceito como limitação conhecida no MVP; se precisar de limite de verdade entre instâncias, é contador externo (Upstash Redis ou Postgres), não decisão tomada ainda.
  - **Cache de JWKS em memória (`GoogleIdentityProvider`) e o pool de conexão do Postgres são por instância.** Instância fria paga o custo de novo (buscar JWKS, abrir conexão) — não é bug, é o modelo; o `jwks_fallback_ttl` e o `max_connections` devem levar isso em conta, não presumir processo de longa duração.

## Arquitetura do backend

A estrutura é um recorte hexagonal simplificado — porta/adaptador nos limites externos (storage, LLM), pragmático no banco (sem abstrair o SQLx atrás de trait: as macros `query!`/`query_as!` verificadas em compile-time são o maior ganho do SQLx, e escondê-las atrás de um trait genérico jogaria isso fora).

```
src/
  main.rs           # bootstrap: env, pool, storage, adaptador Lambda
  config.rs
  domain/           # tipos do laudo — não sabe HTTP, não sabe SQL
    mod.rs  user.rs  report.rs  assessment.rs  circuit.rs  image.rs
  auth/             # porta (IdentityProvider) + cripto — não sabe HTTP, não sabe SQL
    mod.rs  google.rs  password.rs  token.rs
  storage/          # porta (ObjectStorage) + adaptador S3-compatible (R2/MinIO)
    mod.rs  s3.rs
  llm/              # porta (TextGenerator) + adaptador de provedor (Groq)
    mod.rs
  http/             # tudo que sabe HTTP, e só o que sabe HTTP
    mod.rs          # AppState + router sob /api/v1
    error.rs        # erro de domínio → status code
    auth/
      mod.rs  routes.rs  queries.rs  schema.rs  middleware.rs
    images/
      mod.rs  routes.rs  queries.rs  schema.rs
```

O teste prático de `http::`: fora dali, nenhum arquivo importa `axum::`. Cada feature futura (`reports`, `circuits`) ganha sua própria pasta em `http/` no mesmo padrão de `images/` — `routes.rs` (handlers), `queries.rs` (único lugar que sabe SQL), `schema.rs` (contrato público; struct `serde` faz o papel do `.schema`, e é candidato natural a virar `docs/api-contract.md`). Sem camada de service: nos handlers de hoje ela seria pass-through vazio — entra por feature quando existir regra de verdade pra esconder (o cálculo de espaço-reserva da NBR é candidato claro).

`http/auth/` tem um quinto arquivo, `middleware.rs`, fora do padrão de 4. O padrão descreve uma feature CRUD; middleware e extractor são infraestrutura transversal que toda feature consome, e enterrá-los em `routes.rs` faria `images` importar `auth::routes`. Não "corrigir".

**O `axum::Router` não sabe onde roda.** Só o `main.rs` sabe se está atrás de `axum::serve` (TCP) ou do adaptador Lambda — nenhum handler, extractor ou middleware muda entre os dois. É o que mantém a troca de plataforma barata.

## Banco de dados em desenvolvimento

```bash
docker compose up -d              # sobe Postgres + MinIO local, cria o bucket de dev sozinho
cp .env.example .env              # aponta pro Postgres/MinIO locais por padrão
sqlx migrate run                  # aplica as migrations (requer DATABASE_URL no .env)
```

`docker-compose.yml` sobe Postgres 16 e MinIO (S3-compatible) local, mais um serviço `storage-init` que cria o bucket de dev. Deploy definitivo do banco é **Neon**; o schema não depende de nenhum recurso exclusivo de provedor específico, então trocar de Postgres gerenciado no futuro não exige migration.

**Em produção, `DATABASE_URL` é o endpoint com pooling do Neon** (host com sufixo `-pooler`, PgBouncer), não a conexão direta. Cada instância de Lambda abre o próprio pool: sob concorrência, conexão direta estoura o limite do Neon com um punhado de invocações simultâneas. Consequência prática: PgBouncer em modo transaction não suporta statement preparado entre transações — se aparecer erro de prepared statement, é isso, e a saída é `statement_cache_capacity=0` na URL, não voltar pra conexão direta. Migration (`sqlx migrate run`) roda contra a **conexão direta**, não o pooler.

**Squash de migrations liberado enquanto não houver deploy real.** Até existir um banco compartilhado (Neon de produção, ou qualquer ambiente que não seja o volume Docker descartável de cada dev), editar uma migration já "aplicada" localmente é seguro — não existe histórico pra quebrar em lugar nenhum além da sua própria máquina. Prefira fundir a mudança na migration existente (normalmente a `0001_initial.sql`) a empilhar `0002`, `0003`... por ajuste pequeno; recrie o volume local depois (`docker compose down -v && docker compose up -d && sqlx migrate run`). **A partir do primeiro `sqlx migrate run` contra o Neon de produção, essa regra se inverte**: migrations passam a ser append-only, sem exceção.

## Verificação — proporcional ao tamanho da mudança

`cargo check` (rodando contra o Postgres local, pras macros `query!`/`query_as!` verificarem) já garante: código compila, tipos batem, nome de coluna/campo existe. Isso cobre sozinho a maioria das mudanças do dia a dia — rename, ajuste de tipo, comentário, refatoração local.

**Query nova ou alterada em `query!`/`query_as!`: rode `cargo sqlx prepare` e commit o `.sqlx/` junto.** Esse diretório é o cache offline das mesmas macros — um JSON por query, indexado pelo hash do SQL — usado quando não há banco disponível (build sem Docker de pé, `cargo lambda build` do binário de deploy). `cargo check` local sempre passa sem isso, porque fala com o banco direto; o esquecimento só aparece depois, no build offline. Não há CI hoje pra pegar automaticamente — a checagem é manual, na hora do PR.

- **Rename de campo/coluna, ajuste de tipo, fix de comentário, mudança dentro de uma feature já testada**: só `cargo check` (com `DATABASE_URL` apontando pro Postgres local). Não precisa subir o servidor nem rodar o fluxo HTTP de novo — o compilador já teria acusado o que quebrou.
- **Feature nova (rota, tabela, integração externa) que não mexe em cookie/sessão**: `cargo run` (contra o Docker Postgres/MinIO) + `curl`/Postman — mais rápido de iterar que `cargo lambda watch`, e já testa migration aplicando do zero, resposta HTTP correta, integração com storage/banco reais.
- **Mudança que envolve cookie, sessão, ou qualquer coisa que passe pela tradução evento↔HTTP do `lambda_http`** (auth é o caso central hoje) **ou mudança estrutural grande** (reorganização de módulos, troca de camada): validar também sob `cargo lambda watch` antes de considerar pronto — é o único jeito de exercitar a tradução real de payload da API Gateway/Function URL, não só o `axum::Router`.

```bash
cargo run                         # loop rápido do dia a dia — axum::serve, TCP direto
cargo lambda watch                # emula o runtime Lambda local, com reload
cargo lambda invoke --data-file evento.json
cargo lambda build --release --arm64   # binário do deploy (Graviton é mais barato)
```

**`cargo lambda watch` não é `cargo run`.** Ele emula o runtime da Lambda e não fala HTTP puro — invocação é via `cargo lambda invoke` com um arquivo de evento no formato API Gateway v2, não `curl` direto numa porta. O que ele exercita é a tradução evento ↔ HTTP do `lambda_http`, não só o `axum::Router`: header multi-valor (`Set-Cookie` de mais de um cookie vira o campo `cookies` do payload, não múltiplos headers) é o ponto onde essa tradução historicamente quebra, e é justamente o que a autenticação depende. Teste que envolve cookie tem que passar por aqui pelo menos uma vez antes de dar a mudança como validada — `cargo run` sozinho não pega esse tipo de regressão. Passo a passo completo dos dois modos em [`docs/manual-testing-guide.md`](docs/manual-testing-guide.md).

## Git

**Nunca rode `git commit` nem `git push` neste repositório por conta própria** — mesmo em checkpoints de uma tarefa maior onde isso seria a convenção padrão. Deixe as mudanças no working tree (staged ou não) e avise que estão prontas pra revisão. **Exceção única**: o usuário pode liberar um commit específico dizendo literalmente **"commite"** no chat — só nesse caso, e só pra aquela mudança.
