# CLAUDE.md

Backend do sistema de Laudos de Engenharia Elétrica. API REST em Rust, banco Postgres. Substitui o monolito Django legado (repositório `gerador`, congelado, referência apenas — **não releia esse repositório**, todo conhecimento de domínio necessário já foi extraído para `docs/` abaixo).

## Repositórios do projeto

| Repositório | Conteúdo |
|---|---|
| `gerador` | Monolito Django legado. Congelado — só consulta se `docs/` daqui não bastar. |
| `raijin` (este) | Backend Rust/Axum + schema e migrations do banco. |
| `itui` | Frontend React/Vite + Design System Sanhauá. Contrato entre os dois é a API REST — documentar em `docs/api-contract.md` conforme os endpoints forem definidos (ainda não existe). |

## Stack

- **Linguagem/framework**: Rust + Axum.
- **Banco**: Postgres (dev local via Docker, ver `docker-compose.yml`; deploy final em aberto — o schema não usa recurso exclusivo de nenhum provedor).
- **Acesso a dados**: SQLx (queries verificadas em compile-time contra o banco).
- **Auth**: caseira — `argon2` para senha, `jsonwebtoken` para JWT, `oauth2` para o fluxo Google. **Não usar Supabase Auth/GoTrue** (decisão: evitar lock-in).
- **Storage de imagens**: upload direto do frontend via URL pré-assinada (o backend gera a URL, não faz proxy de bytes).
- **IA**: proxy para Groq (Llama-3) via SSE, streaming direto pro frontend.

## Convenções de código

- Tudo em inglês: funções, variáveis, tipos, colunas, rotas. `snake_case` para funções/variáveis/campos, `PascalCase` para tipos/structs/enums, `SCREAMING_SNAKE_CASE` para constantes.
- Português só em conteúdo voltado ao usuário final (mensagens de erro exibidas, conteúdo do laudo) — nunca em código.
- **Não invente nomes de campo.** Toda a nomenclatura do domínio (campos do laudo, seções, enums) vem de [`docs/domain-glossary.md`](docs/domain-glossary.md). Se faltar algo lá, adicione antes de usar em código.

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
- **Auth**: tabela `users` própria (não delegar pro Supabase). Login Google e e-mail/senha convergem pro mesmo usuário quando o e-mail bate.
- **`.env` nunca committado.** Segredos (JWT secret, client secret do Google, `DATABASE_URL`) só via variável de ambiente — o legado tinha credenciais em texto puro no repo, não repetir.

## Banco de dados em desenvolvimento

```bash
docker compose up -d              # sobe o Postgres local
sqlx migrate run                  # aplica as migrations (requer DATABASE_URL no .env)
```

`docker-compose.yml` sobe Postgres 16 local. Deploy definitivo do banco é decisão futura — o schema não depende de nenhum recurso exclusivo de provedor específico.
