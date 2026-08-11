# Contrato da API — `raijin` ↔ `itui`

Contrato REST entre o backend (`raijin`, Rust/Axum) e o frontend (`itui`, React/Vite). Os dois
repositórios **não compartilham tipos**: este documento é a única especificação comum. Toda rota
nova entra aqui **antes** de existir em código.

Nomenclatura de campo vem de [`domain-glossary.md`](domain-glossary.md) — não inventar nomes.
Valores de enum normativo vêm de [`nbr-5410-choices.json`](nbr-5410-choices.json); categorias de
achado, de [`findings-taxonomy.md`](findings-taxonomy.md). Nenhuma dessas listas é duplicada aqui.

---

## Convenções gerais

| Item | Regra |
|---|---|
| Base | `/api/v1` |
| Autenticação | `Authorization: Bearer <access_token>` em tudo, exceto `/api/v1/auth/*` e `/tasks/*` |
| Corpo | `Content-Type: application/json` (exceto o upload de imagem, que vai direto pro bucket) |
| Data/hora | ISO-8601 em UTC (`2026-08-07T14:30:00Z`) |
| Números decimais | **string JSON** (`"12.50"`), não `number` |
| Campos JSON | `snake_case` no fio; o `itui` converte pra `camelCase` na borda |

### Por que decimal é string

Medição elétrica é `numeric` no Postgres e `rust_decimal::Decimal` no Rust justamente pra não
perder precisão. Serializar como `number` JSON entregaria o valor ao `double` do JavaScript e
desfaria a garantia no último metro. O frontend trata como string e só converte pra exibir.

### Formato de erro

Toda falha **tratada pelo handler** responde o mesmo envelope, com mensagem em pt-BR pronta pra
exibição (a exceção está logo abaixo, em "Corpo que nem chega ao handler"):

```json
{ "error": "Laudo não encontrado." }
```

| Status | Quando | Variante de `ApiError` |
|---|---|---|
| `422` | corpo bem formado, conteúdo inválido | `Unprocessable` |
| `401` | token ausente, expirado ou inválido | `Unauthorized`, `InvalidCredentials`, `Token` |
| `404` | recurso não existe **ou não é do usuário** | `NotFound` |
| `409` | conflito de estado (ex.: e-mail já cadastrado) | `Conflict` |
| `500` | falha de banco, storage ou provedor externo | `Database`, `Storage`, `Identity`, `Password` |

Causa de erro `500` é registrada no log do servidor e **nunca** volta no corpo.

### Dois `422` diferentes: só um traz mensagem exibível

Validação **semântica** — campo em branco, valor fora do domínio, regra de negócio — roda no handler
e responde `422` com o envelope normal e mensagem em pt-BR pronta pra exibir.

Validação **estrutural** — JSON malformado, campo obrigatório ausente, tipo errado (`"current":
"abc"`) — é rejeitada pelo extractor `Json` do axum **antes** do handler. Também é `422`, mas o
corpo é **texto puro em inglês** (`Failed to deserialize the JSON body into the target type: ...`),
não o envelope.

Consequência prática pro `itui`: o parse de erro não pode assumir corpo JSON. Tente `res.json()` e
leia `.error`; se falhar, use mensagem própria do frontend — o texto do axum serve pra depurar,
nunca pra exibir. Na prática o caso estrutural só aparece se o frontend montar o corpo errado, o que
é bug de front, não entrada de usuário.

### Autorização: recurso de terceiro responde `404`, nunca `403`

Toda rota com `{report_id}` valida que o laudo pertence ao usuário autenticado. Se não pertencer,
a resposta é **`404 Not Found`** com a mesma mensagem de um laudo inexistente.

`403 Forbidden` seria um vazamento: confirmaria ao cliente que aquele UUID existe e pertence a
outra pessoa. `404` não distingue "não existe" de "não é seu" — o cliente não aprende nada sobre
laudos alheios. A regra vale igualmente para sub-recursos (`/circuits`, `/images`, `/generate`):
a checagem de posse é do **laudo pai**, feita antes de qualquer consulta ao sub-recurso.

No backend isso é o helper `require_ownership` (ver `src/http/reports/routes.rs`), chamado no topo
de todo handler que recebe `{report_id}`.

### Seções não preenchidas

As quatro seções tipadas do laudo (`inspection_planning`, `external_influences`,
`qualitative_assessment`, `quantitative_assessment`) chegam como **`null`** enquanto não forem
preenchidas — nunca como `{}`. Cada uma tem todos os campos obrigatórios, então objeto vazio não
seria um valor válido da seção. O frontend usa `null` como "esta etapa do wizard ainda não foi
concluída".

`document_content` é a exceção: nasce `{}` (árvore vazia do editor TipTap) e nunca é `null`.

---

## Autenticação — `/api/v1/auth/*`

As únicas rotas sem `Authorization: Bearer`. Três caminhos criam sessão (`register`, `login`,
`google`) e todos respondem o mesmo par: **access token no corpo, refresh token no cookie**.

### O par de tokens

| | Onde vive | Validade | Renovação |
|---|---|---|---|
| Access token | corpo da resposta, guardado em memória pelo `itui` | 15 min | via `/auth/refresh` |
| Refresh token | cookie `refresh_token`, `httpOnly` | 30 dias | rotacionado a cada uso |

O refresh token **nunca aparece no corpo** e o JavaScript do `itui` não o lê — é `httpOnly` de
propósito, para que XSS não consiga exfiltrar a sessão longa. Guardar o access token em memória
(não em `localStorage`) é a outra metade dessa decisão.

Atributos do cookie: `HttpOnly; Secure; SameSite=None; Path=/api/v1/auth`. O `Path` restrito faz o
cookie não ser enviado nas rotas de laudo — só as de auth precisam dele. `SameSite=None` é o que
permite o `itui` em outro domínio mandar o cookie, e exige `Secure`; em dev local por `http://`,
o navegador pode recusar o cookie — use o Postman ou sirva o front por HTTPS.

**Resposta de sessão** (as três rotas de criação e o `refresh`):

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "token_type": "Bearer",
  "expires_in": 900,
  "user": {
    "id": "1b2d4f60-...",
    "email": "engenheiro@ufpb.br",
    "google_id": null,
    "avatar_url": null,
    "created_at": "...",
    "updated_at": "..."
  }
}
```

`expires_in` é em segundos: o `itui` agenda a renovação por ele, em vez de decodificar o JWT no
navegador. `password_hash` nunca é serializado.

### `POST /api/v1/auth/register`

```json
{ "email": "engenheiro@ufpb.br", "password": "senha123456" }
```

`201 Created` com a resposta de sessão. O e-mail é normalizado (`trim` + minúsculas) nos três
pontos de entrada — sem isso o vínculo de conta por e-mail nunca dispararia.

**Erros**: `409` em duas variantes de mensagem — `"E-mail já cadastrado."`, ou
`"Esta conta usa login pelo Google. Entre com o Google."` se o e-mail já existe vinculado ao
Google. Também `422`, `429`.

### `POST /api/v1/auth/login`

```json
{ "email": "engenheiro@ufpb.br", "password": "senha123456" }
```

`200 OK` com a resposta de sessão.

**Erros**: `401 "E-mail ou senha inválidos."` — **a mesma mensagem** para e-mail inexistente e
senha errada. O backend ainda verifica um hash-isca quando o usuário não existe, para o tempo de
resposta não virar oráculo de "esse e-mail tem conta". O `itui` não deve tentar distinguir os dois
casos na UI. Também `422`, `429`.

### `POST /api/v1/auth/google`

```json
{ "id_token": "<ID Token do Google Identity Services>" }
```

O `itui` usa o Google Identity Services e manda o **ID Token**; o backend só verifica a assinatura
(RS256, contra o JWKS público). Não há authorization-code flow nem redirect — decisão fechada.

`200 OK` com a resposta de sessão. Login Google e e-mail/senha convergem para o **mesmo usuário**
quando o e-mail bate.

**Takeover de conta:** se o e-mail já tinha conta com senha, o Google (verificado) assume, e **todos
os refresh tokens daquele usuário são revogados** — quem tivesse pré-registrado o e-mail não fica
com sessão viva. Na prática, para o `itui`: uma sessão aberta em outro dispositivo pode morrer nesse
momento; trate `401` no refresh como "faça login de novo", sempre.

**Erros**: `401` (ID Token inválido, ou e-mail não verificado no Google), `503` se o JWKS da Google
estiver fora do ar — token ruim é problema do cliente, JWKS indisponível não é. Também `422`, `429`.

### `POST /api/v1/auth/refresh`

Sem corpo. O refresh token vai no cookie, então o `fetch` do `itui` precisa de
`credentials: "include"`.

`200 OK` com uma resposta de sessão nova, e um `Set-Cookie` com **outro** refresh token: cada uso
rotaciona.

**Detecção de reuso.** Se um refresh token já rotacionado for apresentado de novo, isso indica cópia
vazada: a cadeia inteira daquele usuário é revogada e a resposta é `401`. Para não quebrar o caso
legítimo de duas abas renovando ao mesmo tempo, existe uma **janela de graça de 10 segundos** em que
o token antigo ainda é aceito. Fora dela, `401` e sessão encerrada.

**Erros**: `401` (cookie ausente, token inválido, expirado ou reusado fora da graça), `429`.

### `POST /api/v1/auth/logout`

Sem corpo. `204 No Content`, **idempotente**: responde `204` tenha ou não encontrado a sessão —
senão o logout viraria oráculo de "esse token existe". A resposta traz um `Set-Cookie` que remove o
`refresh_token` (`Max-Age=0`, com o mesmo `Path=/api/v1/auth`).

Só a sessão daquele cookie é encerrada; outros dispositivos seguem logados.

---

## Objeto `Report`

Retornado por todas as rotas de laudo.

```json
{
  "id": "9f1c3a7e-...",
  "author_id": "1b2d4f60-...",
  "location_code": "CCHLA-102",
  "inspected_at": "2026-08-07T14:30:00Z",
  "ambient_temperature_c": 31,
  "weather_conditions": "Ensolarado",
  "responsible_parties": ["Filipe Coutinho", "João Silva"],
  "status": "draft",
  "inspection_planning": null,
  "external_influences": null,
  "qualitative_assessment": null,
  "quantitative_assessment": null,
  "document_content": {},
  "created_at": "2026-08-07T14:35:12Z",
  "updated_at": "2026-08-07T14:35:12Z"
}
```

`status`: `draft` | `in_review` | `approved` | `archived`.
`location_code`: padrão `BLOCO-SALA`, regex `[A-Z]{2,}-[A-Z]{0,}[0-9]{2,}` (`CCHLA-102`, `CI-T02`).

---

## `POST /api/v1/reports` — criar laudo

Cria o laudo com os campos de identidade (§1 do glossário). As seções nascem vazias e são
preenchidas depois, uma a uma, pelos `PATCH` de seção.

**Request**

```http
POST /api/v1/reports
Authorization: Bearer <access_token>
Content-Type: application/json
```

```json
{
  "location_code": "CCHLA-205",
  "inspected_at": "2026-08-07T14:30:00Z",
  "ambient_temperature_c": 31,
  "weather_conditions": "Ensolarado",
  "responsible_parties": ["Filipe Coutinho"]
}
```

| Campo | Tipo | Obrigatório |
|---|---|---|
| `location_code` | string, regex acima | sim |
| `inspected_at` | timestamp ISO-8601 | sim |
| `ambient_temperature_c` | int \| null | não |
| `weather_conditions` | string \| null | não |
| `responsible_parties` | string[] | não (default `[]`) |

**Auto-preenchimento por bloco.** O prefixo antes do primeiro `-` do `location_code` é o *bloco*.
Se o autor já tiver outro laudo no mesmo bloco com `inspection_planning` preenchido, a seção
inteira (os 17 campos da §2) é copiada do laudo mais recente (`ORDER BY created_at DESC LIMIT 1`).
A busca é sempre restrita ao próprio autor — nunca copia planejamento de laudo de outra pessoa.
Sem laudo anterior no bloco, `inspection_planning` vem `null`.

**Response `201 Created`** — o `Report` completo, mais um campo de aviso pro frontend:

```json
{
  "...": "campos do Report",
  "inspection_planning": { "professional_qualification": "...", "...": "..." },
  "planning_autofilled": true
}
```

`planning_autofilled` existe pra que a UI possa avisar *"preenchemos o planejamento com base no
laudo anterior deste bloco — confira"*. Nunca é preenchimento silencioso: os dados de segurança
copiados precisam ser revalidados pelo profissional.

**Erros**: `422` (`location_code` em branco, campo ausente, data inválida), `401`.

---

## `GET /api/v1/reports` — listar laudos do usuário

Retorna apenas laudos do autor autenticado. Sem paginação explícita não há resposta — `limit`
sempre se aplica.

**Query params**

| Param | Tipo | Default | Nota |
|---|---|---|---|
| `status` | enum de `report_status` | — | filtro exato |
| `location_prefix` | string | — | filtra pelo bloco (`CCHLA` casa `CCHLA-102`, `CCHLA-205`) |
| `limit` | int 1..100 | 20 | |
| `offset` | int ≥ 0 | 0 | |

**Response `200 OK`** — array de `Report` **sem as seções JSONB** (payload de listagem é leve;
para as seções, buscar o laudo individual):

```json
[
  {
    "id": "9f1c3a7e-...",
    "location_code": "CCHLA-102",
    "inspected_at": "2026-08-07T14:30:00Z",
    "status": "draft",
    "created_at": "...",
    "updated_at": "..."
  }
]
```

---

## `GET /api/v1/reports/{report_id}` — laudo completo

**Response `200 OK`** — o `Report` com todas as seções, mais os circuitos já embutidos (o wizard
carrega a Parte III junto; poupa um round-trip):

```json
{
  "...": "campos do Report",
  "circuits": [ { "...": "objeto Circuit" } ],
  "spare_circuits": { "circuit_count": 13, "required": 4 }
}
```

Imagens **não** vêm embutidas: elas exigem assinar uma URL de leitura por item, o que é caro e tem
validade curta. Buscar em `GET /reports/{report_id}/images`.

### `spare_circuits` — espaço de reserva calculado (NBR 5410 6.5.4.7)

Campo **derivado, somente leitura**: recalculado a cada resposta a partir do número real de
circuitos do laudo. Não é gravado no banco — circuito novo muda a exigência, e um valor congelado
ficaria mentindo em silêncio.

| Circuitos | `required` |
|---|---|
| 0 | `null` — laudo sem circuito cadastrado, nada a exigir ainda |
| 1 a 6 | 2 |
| 7 a 12 | 3 |
| 13 a 30 | 4 |
| N > 30 | 0,15 × N, arredondado pra cima |

Não confundir com `spare_circuit_capacity` da §4, que é a **faixa declarada pelo engenheiro** na
avaliação qualitativa. O legado só guardava essa escolha e descartava a saída da tabela normativa;
aqui os dois convivem — o declarado e o calculado. Divergir entre eles é informação para a UI
apresentar, **não** um veredito de conformidade que o backend emita.

**Erros**: `401`, `404` (inexistente **ou de outro usuário**).

---

## `PATCH /api/v1/reports/{report_id}` — editar identidade

Body com qualquer subconjunto dos campos de §1, mais `status`. Campo ausente fica inalterado;
campo explicitamente `null` limpa o valor (só nos opcionais).

```json
{ "weather_conditions": "Chuvoso", "status": "in_review" }
```

**Response `200 OK`**: `Report` atualizado. **Erros**: `422`, `401`, `404`.

---

## `DELETE /api/v1/reports/{report_id}`

Apaga o laudo. Circuitos e linhas de imagem caem junto (`ON DELETE CASCADE`); os objetos no bucket
são recolhidos depois pela rotina de limpeza, não no caminho da requisição.

**Response `204 No Content`**. **Erros**: `401`, `404`.

---

## `PATCH` de seção — as cinco seções JSONB

Cinco rotas com o mesmo comportamento. Cada uma recebe a **seção inteira** e a substitui — não é
merge parcial: a seção é uma unidade de validação, e aceitar campo solto permitiria gravar uma
seção incompleta que nenhum consumidor sabe interpretar.

| Rota | Seção | Corpo |
|---|---|---|
| `PATCH /reports/{report_id}/inspection-planning` | §2 | os 17 campos de `InspectionPlanning` |
| `PATCH /reports/{report_id}/external-influences` | §3 | as 22 classes NBR de `ExternalInfluences` |
| `PATCH /reports/{report_id}/qualitative-assessment` | §4 | `QualitativeAssessment` |
| `PATCH /reports/{report_id}/quantitative-assessment` | §5 Partes I e II | `QuantitativeAssessment` |
| `PATCH /reports/{report_id}/document-content` | editor | árvore TipTap (JSON livre) |

Formato dos campos de resposta:

- §2 — booleanos, mais `professional_qualification` (enum), e `identified_hazards` /
  `safety_equipment` / `signage_used` (arrays de enum). Valores em `nbr-5410-choices.json`.
- §3 — cada campo é o código da classe NBR (`"AA4"`, `"AD3"`, …).
- §4 — cada campo é `{ "answer": "yes" | "no" | "partial", "notes": "..." }`. Exceções:
  `spare_circuit_capacity` e `earthing_system_type` são string de escolha única, sem `notes`.
- §5 Parte I — decimais como string (`"127.30"`). Parte II — os 6 ensaios, cada um
  `{ "answer": "yes" | "no", "notes": "..." }`.

Exemplo (§4, recortado):

```json
{
  "has_installation_documentation": { "answer": "partial", "notes": "Só o unifilar de 2018." },
  "spare_circuit_capacity": "7 a 12",
  "earthing_system_type": "TN-S"
}
```

**Response `200 OK`**: `Report` atualizado. **Erros**: `422` (campo faltando, enum desconhecido),
`401`, `404`.

---

## Circuitos — `Circuit`

Um por circuito do quadro de distribuição (§5 Parte III). **Sem limite de quantidade** — o teto de
13 do legado era restrição do template Word, não do domínio.

```json
{
  "id": "3c8e...",
  "report_id": "9f1c3a7e-...",
  "circuit_model": "C1",
  "phase": "A",
  "breaker": "Disjuntor 20A curva C",
  "description": "Tomadas da sala 205",
  "conductor": "2,5 mm²",
  "current": "12.40",
  "created_at": "...",
  "updated_at": "..."
}
```

Só `description` é opcional (`null`); os demais são obrigatórios, `NOT NULL` no banco inclusive.
`current` é decimal em string.

### `GET /api/v1/reports/{report_id}/circuits`

`200 OK` com array de `Circuit`, ordenado por `created_at`.

### `POST /api/v1/reports/{report_id}/circuits`

```json
{ "circuit_model": "C1", "phase": "A", "breaker": "Disjuntor 20A curva C",
  "description": "Tomadas da sala 205", "conductor": "2,5 mm²", "current": "12.40" }
```

`description` pode ser omitido; os outros cinco são obrigatórios.

`201 Created` com o `Circuit`. **Erros**: `422` (campo obrigatório em branco ou ausente, `current`
não numérico), `401`, `404`.

### `PATCH /api/v1/reports/{report_id}/circuits/{circuit_id}`

Subconjunto dos campos acima; ausente fica inalterado. Só `description` aceita `null` explícito para
limpar — os obrigatórios não voltam a ficar vazios. `200 OK` com o `Circuit`.
`404` se o circuito não existe **ou não pertence a esse laudo**.

### `DELETE /api/v1/reports/{report_id}/circuits/{circuit_id}`

`204 No Content`.

---

## Imagens

Já implementado — fluxo de duas etapas com URL pré-assinada:

| Método | Rota |
|---|---|
| `POST` | `/api/v1/reports/{report_id}/images` — cria a linha `pending` e devolve a URL de escrita |
| `POST` | `/api/v1/reports/{report_id}/images/{image_id}/confirm` — confirma contra o bucket |
| `GET` | `/api/v1/reports/{report_id}/images` — lista com URL de leitura assinada |

O upload em si vai do navegador direto pro bucket com `PUT` na `upload_url`, com o
`Content-Type` exatamente igual ao `required_content_type` devolvido — a assinatura cobre esse
header, e divergência resulta em `403` do storage.

`POST .../images` aceita, além de `content_type`, dois campos opcionais e **independentes** um do
outro: `finding_category` (slug de `findings-taxonomy.md` — que não conformidade a foto mostra) e
`report_section` (`inspection_planning` | `external_influences` | `qualitative_assessment` |
`quantitative_assessment` | `circuits` — em qual seção do laudo a foto entra; ausente/`null` = foto
geral, cai no apêndice de imagens ao final do documento). Uma foto pode ter os dois, só um, ou
nenhum. `422` se qualquer um dos dois vier fora da lista aceita.

---

## `GET /api/v1/reports/{report_id}/draft` — modelo padrão do relatório

Monta o texto do laudo a partir das respostas e das imagens confirmadas, sem provedor de IA —
substituição determinística, sucessora do replace de chaves do `template.docx` legado (que não
tinha prosa fixa nenhuma para portar, ver [`report-template.md`](report-template.md)). Responde na
hora, sem depender de serviço externo — é o piso do sistema: se a Groq cair ou a chave faltar, este
caminho continua funcionando.

**Request**

```http
GET /api/v1/reports/{report_id}/draft?image_ids=3c8e...,7a1b...
Authorization: Bearer <access_token>
```

`image_ids` é opcional, CSV de UUID — ausente considera todas as imagens confirmadas com achado.

**Response `200 OK`**

```json
{ "text": "## Avaliação e planejamento da execução\n\n| Item | Descrição | Detalhamento | Observação |\n..." }
```

Markdown, com um `##` por seção do laudo na ordem canônica (planejamento, influências externas,
avaliação qualitativa, avaliação quantitativa, circuitos), seguido do apêndice de imagens gerais.
Seção sem dado preenchido aparece como "Seção não avaliada neste laudo." — nunca omitida.

### O achado fotográfico vem com o `image_id`, não com a URL

Cada achado sai como duas linhas — o marcador da imagem e a legenda:

```markdown
![Condutores energizados expostos e sem proteção](image:3c8e1f42-...)
**(a) Condutores energizados expostos e sem proteção** — Fiação exposta próxima ao jardim
```

O `src` é o esquema **`image:<uuid>`**, não uma URL. A URL de leitura é assinada e de validade
curta, e o `itui` persiste o documento editado em `document_content`: URL embutida apodreceria
dentro do laudo salvo. O frontend resolve o marcador para uma URL fresca (de
`GET .../images`) na hora de renderizar e de exportar — nunca grava a URL resolvida de volta.

A letra `(a)`, `(b)`, `(c)` é a numeração do modelo original e é **por bloco**: reinicia em cada
seção e no apêndice. Não é identificador — para casar foto e legenda, use o `image_id`.

**É também a primeira das duas chamadas do fluxo de IA**: o `itui` carrega este documento no editor
e só então abre o stream do `/generate`, que acrescenta prosa sem tocar nas tabelas.

### As tabelas do modelo legado

Cada seção sai como tabela, com as colunas verbatim de [`report-template.md`](report-template.md) —
`Item | Descrição | Detalhamento | Observação` no planejamento, `Item | Descrição | Classificação |
Tipo | Item da norma NBR 5410` nas influências externas, e assim por diante. A avaliação
quantitativa tem duas sub-tabelas ("Parte I — Medições" e "Parte II — Ensaios realizados"), cada
uma precedida do seu nome em negrito.

Cada célula é uma coluna de verdade: cláusula normativa, classificação e observação **não** vêm
concatenadas no rótulo, justamente pro `itui` não ter que separá-las por regex ao montar a tabela
do TipTap.

**Tabela com cabeçalho de dois níveis sai em HTML, não em Markdown.** É o caso da avaliação
qualitativa (`ASPECTOS OBSERVADOS ATENDEM A NORMA?` abrangendo resposta e observações) e da Parte
II da quantitativa: Markdown GFM não tem `colspan`. Nesses dois blocos o corpo traz `<table>` com
`colspan` no `<th>`; o resto do documento continua Markdown. O conversor do `itui` precisa de
`html: true` (`markdown-it`) para não descartar o bloco, e o TipTap entende `colspan` na célula.

Duas construções do modelo original continuam simplificadas, por não existirem no modelo de
documento do TipTap:

- **As duas grades lado a lado da Parte I** ("Quadro de distribuição" à esquerda, "Circuitos
  terminais" à direita) saem como tabelas em sequência. Lado a lado exigiria um layout de duas
  colunas, que o TipTap não modela.
- **Tabela dentro de célula** (a tabela normativa de espaço-reserva, item 10 da qualitativa) — a
  extensão de tabela do TipTap não aninha. O dado não se perde: o veredito de espaço-reserva já vem
  calculado em linha própria da tabela.

### Classificação e Tipo: desvio consciente do modelo

No `.docx` legado, a coluna `Classificação` da avaliação de influências externas lista **todas** as
opções normativas (`AA1` a `AA8`) e a coluna `Tipo` recebe a escolhida. Aqui é diferente:
`Classificação` traz o código escolhido (`AA5`) e `Tipo` traz a descrição dele
(`Quente (5 ° a 40 °C)`).

O modelo original é um formulário de campo — as opções existem para serem circuladas à mão. Num
laudo entregue, imprimir as 34 classificações que não se aplicam é ruído; no item de influências
eletromagnéticas seriam ~35 linhas por campo. E o mesmo material alimenta o prompt da IA, onde isso
custaria contexto sem acrescentar informação. Se a fidelidade ao formulário virar requisito, o
catálogo já está em [`nbr-5410-choices.json`](nbr-5410-choices.json) e é acréscimo no renderizador,
não mudança de estrutura.

### Markdown, nos dois caminhos

`/draft` e `/generate` saem no mesmo vocabulário de construções — `## ` por seção, `### ` por bloco
de não conformidades, tabela GFM por grade, `**negrito**` em legenda de sub-tabela. O TipTap não
consome Markdown nativamente: o `itui` converte (`markdown-it`/`marked` → `generateJSON`, ou
`tiptap-markdown`) antes de carregar no editor. Tabela exige as extensões `@tiptap/extension-table`
e irmãs — o `StarterKit` não as traz, e sem elas a tabela vira parágrafo solto. Achado fotográfico
exige também `@tiptap/extension-image`, pelo mesmo motivo.

Valor de campo digitado pelo engenheiro vai **escapado** pelo backend (`*`, `_`, `` ` ``, `[`, `]`,
`<`, `~`, `\` e `|`) — uma observação como `Emenda 2*3mm` não vira ênfase, e um `|` no meio do texto
não parte a linha da tabela em duas células. Rótulo e cabeçalho não são escapados: vêm de `docs/`,
não do usuário.

**Erros**: `401`, `404` (laudo não é do usuário), `422` (nenhuma seção preenchida e nenhum achado —
"Preencha ao menos uma seção do laudo antes de gerar o texto.").

---

## `POST /api/v1/reports/{report_id}/generate` — redação por IA (SSE)

Gera, em streaming, **a prosa técnica de cada seção** — não o documento inteiro. As tabelas são do
`/draft`, e só dele, nos dois caminhos. O consumo via SSE é bem diferente do resto da API — ver a
seção de consumo no frontend, abaixo.

### Divisão de trabalho: tabela é do backend, prosa é da IA

O modelo **não reproduz nenhum dado do laudo**. Ele recebe o material e devolve apenas a leitura
técnica: o que os valores indicam, quais itens estão em desacordo com a norma, o que decorre disso.
Quem emite classificação, medição, resultado de ensaio e circuito é o modelo determinístico, igual
nos dois endpoints.

A razão é de fidelidade, não de arquitetura: enquanto pedimos ao modelo que reproduzisse as
tabelas, ele omitia medições ("os valores de tensão e corrente foram medidos", sem os valores),
agrupava classificações distintas numa generalização falsa e perdia linhas inteiras. Nenhuma regra
de prompt elimina essa classe de erro; tirar o dado das mãos dele elimina. **A consequência prática
é que a tabela do `/generate` é byte-idêntica à do `/draft`.**

### Fluxo no frontend: duas chamadas, nesta ordem

1. `GET .../draft` — responde na hora, sem provedor externo. O `itui` já monta o documento completo
   no editor, com todas as tabelas e todos os dados.
2. `POST .../generate` — streama a prosa, seção a seção. Cada trecho é inserido **depois das
   tabelas da seção a que pertence**.

Nada é substituído no fim: as tabelas nascem prontas na etapa 1 e a prosa é acrescentada em volta.
Se o provedor de IA cair no meio, o usuário continua com o laudo determinístico íntegro no editor —
perdeu a redação, não o trabalho. É o que torna concreta, na interface, a regra de que o `/draft` é
o piso do sistema.

**Request**

```http
POST /api/v1/reports/{report_id}/generate
Authorization: Bearer <access_token>
Accept: text/event-stream
Content-Type: application/json
```

```json
{ "image_ids": ["3c8e...", "7a1b..."] }
```

`image_ids` opcional — se ausente, considera todas as imagens confirmadas do laudo.

**Response `200 OK`**

```http
Content-Type: text/event-stream
Cache-Control: no-cache
```

Três tipos de evento:

```
event: token
data: {"section":"qualitative_assessment","text":"A instalação apresenta "}

event: token
data: {"section":"qualitative_assessment","text":"conexões sem isolação adequada"}

event: done
data: {"finish_reason":"stop","total_tokens":412}

event: error
data: {"error":"Provedor de IA indisponível. Tente novamente."}
```

**`section`** diz em que seção do documento aquele trecho entra — é o que permite ao `itui`
encaixar a prosa no lugar certo em vez de empilhar tudo no fim. Os valores são as mesmas chaves de
`report_section` das imagens (`domain::REPORT_SECTIONS`), mais `images` para o apêndice:

| `section` | Título da seção no `/draft` |
|---|---|
| `inspection_planning` | `## Avaliação e planejamento da execução` |
| `external_influences` | `## Avaliação das influências externas da instalação elétrica` |
| `qualitative_assessment` | `## Avaliação qualitativa da instalação elétrica` |
| `quantitative_assessment` | `## Avaliação quantitativa da instalação` |
| `circuits` | `## Circuitos` |
| `images` | `## Imagens do Relatório` |

Os títulos são a âncora que o `itui` usa para localizar a seção no Markdown do `/draft`. São
canônicos (vêm de [`report-template.md`](report-template.md)) e mudam junto com este contrato,
nunca sozinhos.

As seções chegam na ordem da tabela acima, uma de cada vez: todos os `token` de uma seção antes do
primeiro da próxima. Seção marcada "não avaliada" no `/draft` **não recebe prosa** — o modelo é
proibido de inferir conformidade sobre dado ausente.

`total_tokens` é opcional no evento `done` — nem todo provedor reporta uso (Gemini reporta,
depende do modelo; campo ausente quando o adaptador não recebeu essa informação).

**`finish_reason` vale ser olhado, não só logado.** `"stop"` é conclusão normal; `"length"` quer
dizer que o teto de tokens de saída cortou a prosa no meio — a última seção chega incompleta e as
seguintes não chegam. O documento continua íntegro (as tabelas são do `/draft`), mas o `itui`
deveria avisar que a redação foi truncada em vez de apresentá-la como pronta.

Erros que acontecem **antes** do primeiro byte (`401`, `404`, `422` — mesmo `422` de laudo vazio do
`/draft`) vêm como resposta HTTP normal, com o envelope `{"error": "..."}` de sempre — inclusive
`503` se o provedor de IA rejeitar a chamada já na abertura (chave inválida, serviço fora do ar).
Depois que o stream abriu o status já foi enviado, então falha vira `event: error` e o stream
encerra.

Esse `503` na abertura só sai depois de **toda a cascata** de modelos falhar. O backend tenta os
modelos de `LLM_CHAIN` em ordem, e limite estourado (`413`/`429`) ou provedor fora do ar (`5xx`)
faz cair para o seguinte; o `itui` não vê a troca nem precisa saber qual modelo respondeu. Isso
importa porque a cota é por chave e por dia, não por usuário: dois engenheiros gerando ao mesmo
tempo dividem o mesmo balde, e o modelo do topo da cascata esgota com poucas dezenas de laudos.

Erro que **não** é de capacidade (`401` de chave inválida, `404` de modelo inexistente) interrompe
a cascata em vez de percorrê-la: não é falta de capacidade, é configuração errada, e tentar os
outros elos só esconderia o defeito.

A rede só cobre a abertura. Provedor que cai com o stream já aberto continua virando
`event: error`, porque retentar duplicaria o texto já entregue ao editor.

**`location_code` e `responsible_parties` nunca entram no prompt.** O que sobe pro provedor de IA é
o texto das seções do laudo (perguntas e respostas) mais categoria do achado + descrição das
imagens — nada que identifique a edificação ou uma pessoa real. Um laudo fotografa vulnerabilidade
física real de um prédio; associar isso a um endereço ou a um nome num serviço de terceiro é vazar
mapa de vulnerabilidade.

### Consumo no frontend: por que `EventSource` não serve

A API nativa `EventSource` do navegador tem duas limitações fatais aqui:

1. **só faz `GET`** — não há como mandar corpo com `image_ids`;
2. **não aceita headers customizados** — não dá pra enviar `Authorization: Bearer`.

A saída comum (token na query string) está descartada: query string vaza em log de servidor, de
proxy e no histórico do navegador.

Então o consumo é `fetch` + `ReadableStream`, fazendo o parse do protocolo SSE à mão:

```ts
export async function generateReport(
  reportId: string,
  accessToken: string,
  imageIds: string[],
  onToken: (section: string, text: string) => void,
): Promise<void> {
  const response = await fetch(`/api/v1/reports/${reportId}/generate`, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${accessToken}`,
      "Content-Type": "application/json",
      "Accept": "text/event-stream",
    },
    body: JSON.stringify({ image_ids: imageIds }),
  });

  // Erro antes do stream abrir ainda é uma resposta JSON comum.
  if (!response.ok) {
    const { error } = await response.json();
    throw new Error(error);
  }

  const reader = response.body!.pipeThrough(new TextDecoderStream()).getReader();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += value;

    // Um evento SSE termina em linha em branco; o resto fica no buffer
    // aguardando o próximo chunk — um chunk pode cortar um evento no meio.
    let separator: number;
    while ((separator = buffer.indexOf("\n\n")) !== -1) {
      const raw = buffer.slice(0, separator);
      buffer = buffer.slice(separator + 2);

      const event = raw.match(/^event: (.*)$/m)?.[1] ?? "message";
      const data = raw.match(/^data: (.*)$/m)?.[1];
      if (!data) continue;

      const payload = JSON.parse(data);
      if (event === "token") onToken(payload.section, payload.text);
      else if (event === "error") throw new Error(payload.error);
      else if (event === "done") return;
    }
  }
}
```

Alternativa pronta: **[`@microsoft/fetch-event-source`](https://www.npmjs.com/package/@microsoft/fetch-event-source)**,
que resolve o mesmo problema com a assinatura do `fetch` e entrega de brinde reconexão automática,
`AbortController` e callback de retry:

```ts
import { fetchEventSource } from "@microsoft/fetch-event-source";

await fetchEventSource(`/api/v1/reports/${reportId}/generate`, {
  method: "POST",
  headers: { Authorization: `Bearer ${accessToken}`, "Content-Type": "application/json" },
  body: JSON.stringify({ image_ids: imageIds }),
  onmessage(msg) {
    if (msg.event === "token") {
      const { section, text } = JSON.parse(msg.data);
      onToken(section, text);
    }
  },
  onerror(err) { throw err; }, // sem throw, a lib tenta reconectar pra sempre
});
```

Escolha do `itui` a fazer na implementação. O contrato do backend é o mesmo nos dois casos.

### Encaixando a prosa no documento

O `onToken` recebe a chave da seção; o `itui` acumula por chave e insere o texto **depois das
tabelas daquela seção**, localizando-a pelo título canônico da tabela acima. Duas regras que
poupam retrabalho:

- **Acumule por seção antes de inserir**, ou insira num nó de parágrafo criado uma vez por seção e
  atualizado a cada token — não crie um nó por token, senão o histórico de undo do TipTap fica
  inutilizável (um Ctrl+Z por pedaço de palavra).
- **Não toque nas tabelas.** Elas vieram do `/draft` e são a fonte da verdade dos dados; o stream
  só acrescenta parágrafos.

---

## `POST /tasks/cleanup-sessions` — limpeza de sessão vencida

**Fora de `/api/v1`** e fora do fluxo do `itui`: é uma rota de manutenção, chamada por um agendador
externo (AWS EventBridge Scheduler em produção), não pelo frontend. Está documentada aqui porque
existe no router e responde HTTP como qualquer outra.

Autenticação é por header próprio, não por Bearer:

```http
POST /tasks/cleanup-sessions
X-Task-Token: <TASK_TOKEN>
```

O token é comparado em **tempo constante** e vem da variável de ambiente `TASK_TOKEN` — é
credencial de máquina, não de usuário, então não passa pela tabela `users` nem emite sessão.

**Response `200 OK`**:

```json
{ "deleted": 42 }
```

Apaga as linhas de `refresh_tokens` já vencidas e devolve quantas saíram. Idempotente: rodar duas
vezes seguidas simplesmente devolve `0` na segunda.

**Erros**: `401` (header ausente ou token errado).

Por que endpoint HTTP e não um loop em background: em Lambda não há processo garantido entre
invocações, então tarefa periódica é rota protegida disparada por agendador externo.
