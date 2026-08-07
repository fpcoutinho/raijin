<div align="center">
  <!-- <img src="docs/assets/logo.svg" alt="Raijin" width="48" /> -->

  # Raijin ⚡

  Backend de alta performance para o automatizador de **Laudos de Engenharia Elétrica**.  
  API REST em **Rust + Axum**, focada na NBR 5410, streaming de IA via SSE e arquitetura Thin Backend.

  ![Rust](https://img.shields.io/badge/Rust-1.80+-orange?logo=rust&logoColor=white&labelColor=18181B)
  ![Axum](https://img.shields.io/badge/Axum-0.7-E05D44?logo=rust&logoColor=white&labelColor=18181B)
  ![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-4169E1?logo=postgresql&logoColor=white&labelColor=18181B)
  ![NBR 5410](https://img.shields.io/badge/NBR_5410-Compliant-00C853?labelColor=18181B)
  ![License](https://img.shields.io/badge/Licen%C3%A7a-Propriet%C3%A1ria-red?labelColor=18181B)
</div>

---

## 📌 Visão Geral

O Raijin atua como um **Thin Backend** (backend magro). Ele é responsável pelo CRUD de dados, validação de autenticação, emissão de URLs pré-assinadas para mídia e proxy de Inteligência Artificial via SSE.

### Ecossistema de Repositórios

| Repositório | Descrição |
|---|---|
| `gerador` | Monolito Django legado. Congelado, mantido apenas para consulta histórica. |
| **`raijin`** *(este)* | Backend em Rust/Axum + schema e migrations do banco Postgres. |
| [`itui`](../itui) | Frontend React/Vite + Design System Sanhauá. Comunica-se com o Raijin via API REST. |

## 🛠️ Stack Tecnológica

- **Linguagem & Framework:** Rust + Axum
- **Banco de Dados:** PostgreSQL (via SQLx com verificação de queries em compile-time)
- **Autenticação:** Custom in-house (`argon2` para senhas, `jsonwebtoken` para JWT, `oauth2` para Google OAuth2) sem lock-in com BaaS.
- **Armazenamento de Imagens:** Estratégia de Presigned URL. O backend gera a URL assinada de curta duração e o frontend realiza o upload diretamente para o storage, sem proxy de bytes no servidor. Provedor: Cloudflare R2 em produção, MinIO localmente em dev — protocolo S3-compatible, único ponto do código específico de provedor (`storage::ObjectStorage`). Bucket privado; leitura também por URL assinada.
- **Inteligência Artificial:** Proxy para a API da Groq (Llama 3.3 70B / GPT-OSS 120B, free tier) utilizando SSE (*Server-Sent Events*) para streaming de pareceres técnicos em tempo real. Isolado atrás do trait `llm::TextGenerator` — trocar de provedor (ex. Gemini) não toca no resto do backend.

## 📐 Arquitetura e Decisões de Modelagem

- **Modelagem Híbrida do Banco:** 
  - Colunas relacionais para identidade, busca, ordenação, chave estrangeira e auditoria.
  - Colunas `JSONB` por seções temáticas do laudo (`inspection_planning`, `external_influences`, `qualitative_assessment`, `quantitative_assessment`, `document_content`).
  - `circuits` e `report_images` são tabelas relacionais 1:N próprias.
- **Sem limite de circuitos:** Diferente do legado que limitava a 13 circuitos por restrição de template Word, a nova arquitetura itera livremente.
- **Avaliação Qualitativa Ternária:** Suporta `Sim`, `Não` e `Parcialmente`. Ensaios quantitativos são binários (`Sim`/`Não`).
- **Precisão Numérica:** Medições críticas usam `numeric` no Postgres e `rust_decimal::Decimal` no Rust, evitando perda de precisão de ponto flutuante.
- **Cálculo de Espaço-Reserva:** O campo `spare_circuit_capacity` executa e valida o cálculo exato da NBR 5410 (item 6.5.4.7), não apenas salvando uma faixa de texto estática.

### Arquitetura do backend

A estrutura é um recorte hexagonal simplificado — porta/adaptador nos limites externos (storage, LLM), pragmático no banco (sem abstrair o SQLx atrás de trait: as macros `query!`/`query_as!` verificadas em compile-time são o maior ganho do SQLx, e escondê-las atrás de um trait genérico jogaria isso fora).
`axum::` não existe fora de `http::`. Cada feature ganha sua própria pasta em `http/` no mesmo padrão de `images/`: `routes.rs` (handlers), `queries.rs` (único lugar que sabe SQL), `schema.rs` (contrato público; struct `serde` faz o papel do `.schema`).

```
src/
  main.rs           # bootstrap: env, pool, storage, serve
  config.rs
  domain/           # tipos do laudo — não sabe HTTP, não sabe SQL
    mod.rs  user.rs  report.rs  assessment.rs  circuit.rs  image.rs
  storage/          # porta (ObjectStorage) + adaptador S3-compatible (R2/MinIO)
    mod.rs  s3.rs
  llm/              # porta (TextGenerator) + adaptador de provedor (Groq)
    mod.rs
  http/             # tudo que sabe HTTP, e só o que sabe HTTP
    mod.rs          # AppState + router sob /api/v1
    error.rs        # erro de domínio → status code
    images/
      mod.rs  routes.rs  queries.rs  schema.rs
```

## 📚 Documentação de Domínio

Todo o conhecimento normativo e de regras de negócio extraído do sistema legado reside na pasta `docs/`. Estes arquivos são a fonte canônica da verdade para a modelagem de dados e engenharia de prompts:

- [`docs/domain-glossary.md`](docs/domain-glossary.md) — Mapa dos ~90 campos do laudo, rótulos pt-BR e regras.
- [`docs/nbr-5410-choices.json`](docs/nbr-5410-choices.json) — Listas normativas oficiais da NBR 5410.
- [`docs/nbr-5410-tests.md`](docs/nbr-5410-tests.md) — Ensaios da avaliação quantitativa e regra de cálculo do espaço-reserva.
- [`docs/findings-taxonomy.md`](docs/findings-taxonomy.md) — Taxonomia de não conformidades (5 categorias) e base para few-shot do prompt da IA.
- `docs/api-contract.md` — Contrato de endpoints REST compartilhado com o frontend. *(Ainda não existe — nasce junto com os endpoints no Step 4 da migração.)*

## 🚀 Rodando Localmente

### Pré-requisitos
- Rust toolchain (Cargo)
- Docker e Docker Compose
- CLI do SQLx (`cargo install sqlx-cli --no-default-features --features postgres`)

### Setup do Ambiente

1. **Suba o Postgres e o storage (MinIO) locais:**
   ```bash
   docker compose up -d
   ```
   Cria também o bucket de dev automaticamente (serviço `storage-init`).

2. **Configure as variáveis de ambiente:**
   ```bash
   cp .env.example .env
   ```

3. **Aplique as migrations:**
   ```bash
   sqlx migrate run
   ```

4. **Rode o servidor** (com hot reload, via `cargo-watch`):
   ```bash
   cargo watch -x run
   ```
   Ou sem hot reload:
   ```bash
   cargo run
   ```

## 🔒 Licença e Propriedade

Este projeto é um software proprietário e de uso confidencial. Todos os direitos sobre o código-fonte, arquitetura, design e documentação são reservados a **Filipe Paulo Coutinho**. 

O acesso ao repositório não concede nenhuma licença de uso, cópia, modificação ou redistribuição por terceiros sem autorização prévia por escrito.
