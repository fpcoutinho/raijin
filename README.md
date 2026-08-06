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
- **Armazenamento de Imagens:** Estratégia de Presigned URL. O backend gera a URL assinada de curta duração e o frontend realiza o upload diretamente para o provedor de storage (S3/Supabase Storage/R2), sem proxy de bytes no servidor.
- **Inteligência Artificial:** Proxy para a API da Groq (Llama-3) utilizando SSE (*Server-Sent Events*) para streaming de pareceres técnicos em tempo real.

## 📐 Arquitetura e Decisões de Modelagem

- **Modelagem Híbrida do Banco:** 
  - Colunas relacionais para identidade, busca, ordenação, chave estrangeira e auditoria.
  - Colunas `JSONB` por seções temáticas do laudo (`inspection_planning`, `external_influences`, `qualitative_assessment`, `quantitative_assessment`, `document_content`).
  - `circuits` e `report_images` são tabelas relacionais 1:N próprias.
- **Sem limite de circuitos:** Diferente do legado que limitava a 13 circuitos por restrição de template Word, a nova arquitetura itera livremente.
- **Avaliação Qualitativa Ternária:** Suporta `Sim`, `Não` e `Parcialmente`. Ensaios quantitativos são binários (`Sim`/`Não`).
- **Precisão Numérica:** Medições críticas usam `numeric` no Postgres e `rust_decimal::Decimal` no Rust, evitando perda de precisão de ponto flutuante.
- **Cálculo de Espaço-Reserva:** O campo `spare_circuit_capacity` executa e valida o cálculo exato da NBR 5410 (item 6.5.4.7), não apenas salvando uma faixa de texto estática.

## 📚 Documentação de Domínio

Todo o conhecimento normativo e de regras de negócio extraído do sistema legado reside na pasta `docs/`. Estes arquivos são a fonte canônica da verdade para a modelagem de dados e engenharia de prompts:

- [`docs/domain-glossary.md`](docs/domain-glossary.md) — Mapa dos ~90 campos do laudo, rótulos pt-BR e regras.
- [`docs/nbr-5410-choices.json`](docs/nbr-5410-choices.json) — Listas normativas oficiais da NBR 5410.
- [`docs/nbr-5410-tests.md`](docs/nbr-5410-tests.md) — Ensaios da avaliação quantitativa e regra de cálculo do espaço-reserva.
- [`docs/findings-taxonomy.md`](docs/findings-taxonomy.md) — Taxonomia de não conformidades (5 categorias) e base para few-shot do prompt da IA.
- [`docs/api-contract.md`](docs/api-contract.md) — Contrato de endpoints REST compartilhado com o frontend.

## 🚀 Rodando Localmente

### Pré-requisitos
- Rust toolchain (Cargo)
- Docker e Docker Compose
- CLI do SQLx (`cargo install sqlx-cli --no-default-features --features postgres`)

### Setup do Ambiente

1. **Suba o banco de dados Postgres local:**
   ```bash
   docker compose up -d
   ```

## 🔒 Licença e Propriedade

Este projeto é um software proprietário e de uso confidencial. Todos os direitos sobre o código-fonte, arquitetura, design e documentação são reservados a **[Seu Nome Completo]**. 

O acesso ao repositório não concede nenhuma licença de uso, cópia, modificação ou redistribuição por terceiros sem autorização prévia por escrito.
