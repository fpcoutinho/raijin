# Guia de testes manuais — auth, rate limiting e cleanup-sessions

Passo a passo pra testar na mão os fluxos que a migração pra Lambda toca:
registro/login/Google, refresh com janela de graça, logout, rate limiting,
rota protegida por token, e o endpoint de limpeza de sessão. Cobre tanto
`cargo run` (loop rápido do dia a dia) quanto `cargo lambda watch` (runtime
Lambda emulado, o que de fato valida antes de um deploy).

Todos os comandos assumem PowerShell no Windows, no diretório raiz do
repositório (`C:\Users\fpcou\workdir\raijin`). Onde `curl` aparece, é o
`curl.exe` que já vem com o Windows 10/11 (não o alias do PowerShell) —
confirme com `curl.exe --version`; se der erro, force `curl.exe` no lugar de
`curl` em todos os comandos abaixo.

---

## 1. Setup do banco e do storage

### 1.1. Subir os containers

```bash
docker compose up -d
```

Isso sobe três serviços: `db` (Postgres 16, porta 5432), `storage` (MinIO,
portas 9000 API / 9001 console web) e `storage-init` (roda uma vez, cria o
bucket `raijin` no MinIO e sai — normal ele aparecer como "Exited" depois).

Confirme que `db` e `storage` estão de pé:

```bash
docker compose ps
```

Espera-se `STATUS` = `Up` pros dois (não pro `storage-init`, que termina).

### 1.2. Configurar o `.env`

```bash
cp .env.example .env
```

Abra `.env` e preencha os campos vazios:

| Variável | Valor pra dev local | Observação |
|---|---|---|
| `JWT_SECRET` | qualquer string aleatória com **32+ caracteres** | ex.: gere com `openssl rand -base64 48` (ou qualquer gerador de senha) |
| `TASK_TOKEN` | qualquer string aleatória com **32+ caracteres** | mesma regra do `JWT_SECRET`. Autentica o endpoint de limpeza de sessão (seção 6) |
| `GOOGLE_CLIENT_ID` | `dev-placeholder-client-id.apps.googleusercontent.com` | serve pra o servidor subir; o fluxo de login Google real não dá pra testar sem um Client ID de verdade do Google Cloud Console (ver seção 3.3) |
| `STORAGE_SECRET_ACCESS_KEY` | `raijin123` | já bate com `MINIO_ROOT_PASSWORD` do `docker-compose.yml` |

O resto (`DATABASE_URL`, `STORAGE_ENDPOINT`, `STORAGE_BUCKET`,
`STORAGE_ACCESS_KEY_ID`, `STORAGE_REGION`, `CORS_ALLOWED_ORIGINS`) já vem
certo no `.env.example` pra apontar pros containers locais — não precisa
mexer, a menos que você tenha mudado portas no `docker-compose.yml`.

Se algum campo obrigatório ficar vazio ou `JWT_SECRET`/`TASK_TOKEN` tiver
menos de 32 caracteres, o servidor recusa subir com
`configuração inválida — confira as variáveis de ambiente contra .env.example`.

### 1.3. Rodar as migrations

```bash
sqlx migrate run
```

Se der erro de comando não encontrado, instale o `sqlx-cli` primeiro:

```bash
cargo install sqlx-cli --version "^0.9" --no-default-features --features postgres,rustls
```

`sqlx migrate run` lê `DATABASE_URL` do `.env` automaticamente (via
`dotenvy` — na real quem lê é o processo do `sqlx-cli`, que também respeita
um `.env` no diretório atual).

---

## 2. Subindo o servidor

### 2.1. Modo normal (`cargo run`)

```bash
cargo run
```

A primeira compilação demora (todas as dependências do zero) — é normal
levar alguns minutos na primeira vez, principalmente com o `lambda_http` no
grafo desde a migração. Compilações seguintes são incrementais e rápidas.

Quando aparecer esta linha, o servidor está pronto:

```
INFO raijin: raijin ouvindo em 0.0.0.0:3000
```

Deixe esse terminal aberto — os testes das seções 3 a 6 usam `curl` num
outro terminal, contra `http://localhost:3000`.

**Antes de repetir os testes**, se você já tinha um `cargo run` de uma
sessão anterior aberto, feche-o primeiro (`Ctrl+C` no terminal dele, ou
`Get-Process raijin | Stop-Process` no PowerShell) — dois processos não
conseguem escutar a mesma porta 3000 ao mesmo tempo, e o binário antigo
trava o `target\debug\raijin.exe` pro próximo `cargo build`/`cargo run`
sobrescrever (erro "Acesso negado").

### 2.2. Modo runtime Lambda real (`cargo lambda watch`)

Esse modo emula o runtime da AWS Lambda de verdade — é o único jeito de
testar a tradução de `Set-Cookie` que o `lambda_http` faz, que é justamente
onde a autenticação (cookie httpOnly do refresh) historicamente quebra sob
Lambda. Ver seção 7.

Requer o `cargo-lambda` instalado:

```bash
cargo install cargo-lambda
```

E o Zig (usado pro cross-compile) no `PATH` da sessão. Se você instalou o
Zig via `winget install zig.zig` e o `cargo lambda build`/`watch` reclamar
"Failed to find zig", ache o binário e adicione ao `PATH` manualmente:

```powershell
$zigDir = (Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Filter "zig.exe" -Recurse | Select-Object -First 1).DirectoryName
$env:Path = "$zigDir;$env:Path"
cargo lambda watch
```

Diferente do `cargo run`, `cargo lambda watch` **não fala HTTP puro** — ele
sobe um emulador da Runtime API da Lambda em `127.0.0.1:9000` e espera
eventos no formato que a API Gateway/Function URL mandaria. Pra invocar,
use `cargo lambda invoke` com um arquivo de evento (exemplos completos na
seção 7), não `curl` direto na porta 3000.

---

## 3. Fluxo de autenticação por e-mail/senha

Os testes desta seção usam um *cookie jar* do `curl` pra guardar o cookie
`refresh_token` entre requisições, exatamente como um navegador faria.

Abra um terminal novo (deixe o `cargo run` da seção 2.1 rodando no outro) e
rode, um de cada vez:

### 3.1. Registro (`POST /auth/register`)

```bash
curl.exe -i -c cookies.txt -X POST http://localhost:3000/api/v1/auth/register ^
  -H "Content-Type: application/json" ^
  -d "{\"email\":\"teste@exemplo.com\",\"password\":\"senha123456\"}"
```

**Esperado**: `201 Created`. O corpo traz `access_token` (JWT, válido por
15 min — `expires_in: 900`) e o objeto `user`. O header `Set-Cookie` traz o
`refresh_token`, marcado `HttpOnly; SameSite=None; Secure; Path=/api/v1/auth`
— o `curl` salva esse cookie em `cookies.txt` por causa do `-c cookies.txt`.

Repetir o mesmo comando de novo (mesmo e-mail) deve dar `409 Conflict` com
`"E-mail já cadastrado."`.

### 3.2. Login (`POST /auth/login`)

```bash
curl.exe -i -c cookies.txt -X POST http://localhost:3000/api/v1/auth/login ^
  -H "Content-Type: application/json" ^
  -d "{\"email\":\"teste@exemplo.com\",\"password\":\"senha123456\"}"
```

**Esperado**: `200 OK`, mesmo formato de corpo do registro, e um
`refresh_token` **novo** no `Set-Cookie` (login sempre emite uma sessão
nova).

Teste a senha errada:

```bash
curl.exe -i -X POST http://localhost:3000/api/v1/auth/login ^
  -H "Content-Type: application/json" ^
  -d "{\"email\":\"teste@exemplo.com\",\"password\":\"senha-errada\"}"
```

**Esperado**: `401 Unauthorized`, `"E-mail ou senha inválidos."` — a mesma
mensagem genérica tanto pra e-mail inexistente quanto pra senha errada (de
propósito: não dá pra usar o erro como oráculo de "esse e-mail existe").

### 3.3. Login via Google (`POST /auth/google`) — não testável sem Client ID real

```bash
curl.exe -i -X POST http://localhost:3000/api/v1/auth/google ^
  -H "Content-Type: application/json" ^
  -d "{\"id_token\":\"qualquer-coisa\"}"
```

Com o `GOOGLE_CLIENT_ID` placeholder do `.env.example`, isso sempre dá
`401` (`"Não foi possível validar o login pelo Google."`) — o servidor
rejeita porque o token não é um JWT RS256 válido assinado pela Google. Pra
testar esse fluxo de verdade, você precisaria de um Client ID real (Google
Cloud Console) e de um ID Token real emitido pelo Google Identity Services
no `itui` — fora do escopo de um teste manual isolado do backend.

### 3.4. Refresh e a janela de graça de 10 segundos (`POST /auth/refresh`)

```bash
curl.exe -i -b cookies.txt -c cookies.txt -X POST http://localhost:3000/api/v1/auth/refresh
```

**Esperado**: `200 OK`, `access_token` novo, e o `refresh_token` do cookie
é **rotacionado** (`Set-Cookie` traz um valor diferente do anterior). O
`-b cookies.txt -c cookies.txt` lê o cookie salvo e já sobrescreve com o
novo — repita o mesmo comando de novo e cada chamada usa o cookie da
chamada anterior.

Agora o teste que importa — a janela de graça (cobre duas abas do
navegador renovando ao mesmo tempo):

1. Copie o valor do `refresh_token` atual de `cookies.txt` antes de rodar o
   próximo refresh (abra o arquivo, é texto simples).
2. Rode um refresh normal (`-b cookies.txt -c cookies.txt` como acima) —
   isso rotaciona o cookie e invalida o valor que você copiou no passo 1.
3. **Dentro de 10 segundos**, reapresente o cookie **antigo** (o que você
   copiou no passo 1) manualmente:

   ```bash
   curl.exe -i -X POST http://localhost:3000/api/v1/auth/refresh ^
     -H "Cookie: refresh_token=VALOR_ANTIGO_COPIADO"
   ```

   **Esperado**: `200 OK` — a janela de graça deixa passar porque assume
   que são duas abas renovando ao mesmo tempo, não roubo de token.

4. **Espere 10+ segundos** e repita o passo 3 com o mesmo valor antigo.

   **Esperado**: `401 Unauthorized` — passou a janela de graça, o token é
   tratado como reuso (sinal de roubo) e a **cadeia inteira é revogada**.

5. Confirme a revogação: tente um refresh com o cookie **mais recente**
   (o que veio do passo 2, que era válido até agora):

   ```bash
   curl.exe -i -b cookies.txt -X POST http://localhost:3000/api/v1/auth/refresh
   ```

   **Esperado**: `401 Unauthorized` também — mesmo sendo o token "certo",
   a cadeia toda morreu no passo 4. Se isso desse `200`, seria o bug de
   segurança que já foi corrigido nesse fluxo (ver `rotate_refresh_token`
   em `src/http/auth/queries.rs`) voltando.

Depois desse teste, registre um usuário novo (seção 3.1) pra continuar com
um cookie válido nos testes seguintes.

### 3.5. Logout (`POST /auth/logout`) — idempotente

```bash
curl.exe -i -b cookies.txt -X POST http://localhost:3000/api/v1/auth/logout
```

**Esperado**: `204 No Content`, e o `Set-Cookie` de resposta remove o
cookie (`refresh_token=; Path=/api/v1/auth; Max-Age=0; ...`) — repare no
`Path=/api/v1/auth` explícito: sem ele, o navegador não saberia qual cookie
apagar (existe mais de um `refresh_token` possível se os paths não
baterem).

Rode o **mesmo comando de novo**:

**Esperado**: `204 No Content` de novo (idempotente — não vira `401` nem
qualquer outro erro só porque a sessão já não existe mais).

Confirme que a sessão morreu de verdade:

```bash
curl.exe -i -b cookies.txt -X POST http://localhost:3000/api/v1/auth/refresh
```

**Esperado**: `401 Unauthorized`.

---

## 4. Rate limiting em `/auth/*`

As rotas `/api/v1/auth/*` têm um limite de 10 requisições por segundo por
IP (`burst_size: 10`, `period: 1s` — ver `src/http/auth/mod.rs`). Sob
`cargo run` local, sem proxy na frente, o rate limit cai no IP da conexão
TCP (sempre `127.0.0.1`), então **todas** as chamadas de um terminal local
competem pelo mesmo balde.

Dispare 15 requisições rapidinho (PowerShell, não `curl.exe` sequencial —
precisa ser rápido o bastante pra estourar o balde de 1 segundo):

```powershell
1..15 | ForEach-Object -Parallel {
    curl.exe -s -o $null -w "%{http_code}`n" -X POST http://localhost:3000/api/v1/auth/login `
      -H "Content-Type: application/json" -d '{"email":"x@x.com","password":"x"}'
} -ThrottleLimit 15
```

**Esperado**: a maioria `401` (credenciais inválidas — comportamento
normal do endpoint), mas algumas das 15 devem vir `429 Too Many Requests`
depois que o balde de 10/segundo estoura.

Isso confirma que a layer está montada — **não** que ela é uma defesa de
segurança real (o custo do `argon2` é a defesa de verdade contra força
bruta; o rate limit é só amortecedor, e sob Lambda cada instância fria tem
seu próprio balde zerado — ver `CLAUDE.md`).

---

## 5. Rota protegida por token (`AuthUser`)

Qualquer rota fora de `/api/v1/auth` exige `Authorization: Bearer <access_token>`.
Exemplo com a rota de imagens (`GET /api/v1/reports/{report_id}/images`):

```bash
curl.exe -i http://localhost:3000/api/v1/reports/00000000-0000-0000-0000-000000000000/images
```

**Esperado**: `401 Unauthorized` (sem header `Authorization`).

Repita com um `access_token` válido (pegue o de um registro/login recente
na seção 3):

```bash
curl.exe -i http://localhost:3000/api/v1/reports/00000000-0000-0000-0000-000000000000/images ^
  -H "Authorization: Bearer SEU_ACCESS_TOKEN_AQUI"
```

**Esperado**: já passa da autenticação (não é mais `401` por falta de
token) — o resultado exato depende de o `report_id` de exemplo existir ou
não no banco, o que é normal dar erro de outro tipo (não é isso que este
teste valida).

Uma rota inexistente sem token deve dar `404`, não `401` — é o motivo de
`http::router()` usar `route_layer` em vez de `layer` global:

```bash
curl.exe -i http://localhost:3000/api/v1/rota-que-nao-existe
```

**Esperado**: `404 Not Found`.

---

## 6. Endpoint de limpeza de sessão (`POST /tasks/cleanup-sessions`)

Essa rota fica **fora** de `/api/v1` e fora da autenticação de usuário —
ela é pensada pra ser chamada pelo AWS EventBridge Scheduler, autenticada
por um token de máquina (`X-Task-Token`), não por sessão de usuário.

### 6.1. Sem token / token errado

```bash
curl.exe -i -X POST http://localhost:3000/tasks/cleanup-sessions
```

**Esperado**: `401 Unauthorized`.

```bash
curl.exe -i -X POST http://localhost:3000/tasks/cleanup-sessions -H "X-Task-Token: token-errado"
```

**Esperado**: `401 Unauthorized` também.

### 6.2. Preparar uma sessão vencida pra apagar

Esse endpoint só apaga sessões vencidas há **30+ dias** (a folga preserva
rastro de reuso por um tempo). Como você não vai ter uma sessão assim
naturalmente num ambiente de teste, insira uma à mão via `psql` dentro do
container:

Primeiro, pegue o `id` de um usuário existente (registre um pela seção 3.1
se ainda não tiver nenhum):

```bash
docker compose exec db psql -U raijin -d raijin -c "SELECT id, email FROM users LIMIT 5;"
```

Copie um `id` da lista e insira uma linha de `refresh_tokens` já vencida há
60 dias (substitua `SEU_USER_ID_AQUI`):

```bash
docker compose exec db psql -U raijin -d raijin -c "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, created_at) VALUES (gen_random_uuid(), 'SEU_USER_ID_AQUI', '\x00112233', now() - interval '60 days', now() - interval '90 days');"
```

**Esperado**: `INSERT 0 1`.

### 6.3. Rodar a limpeza

```bash
curl.exe -i -X POST http://localhost:3000/tasks/cleanup-sessions -H "X-Task-Token: SEU_TASK_TOKEN_DO_ENV"
```

(`SEU_TASK_TOKEN_DO_ENV` é o valor que você colocou em `TASK_TOKEN` no
`.env`, seção 1.2.)

**Esperado**: `200 OK`, corpo `{"deleted":N}` com `N >= 1` (pode ser maior
que 1 se você já tinha outras sessões vencidas há 30+ dias no banco de
antes).

Confirme que a linha específica que você inseriu sumiu:

```bash
docker compose exec db psql -U raijin -d raijin -c "SELECT count(*) FROM refresh_tokens WHERE token_hash = '\x00112233';"
```

**Esperado**: `count = 0`.

Rode o `curl` da limpeza de novo (mesmo comando): é idempotente, deve dar
`200 {"deleted":0}` se não sobrou nada vencido há 30+ dias.

---

## 6.5. Lendo o texto gerado por IA (`POST .../generate`)

O `/generate` responde SSE: no terminal, o `curl` mostra a sequência de
`event: token` crua, um pedaço de palavra por linha. Os tokens concatenados
**são** o Markdown completo — quem junta é o cliente. Para ver o laudo em vez
do protocolo, filtre as linhas `data:` e concatene os campos `text`:

```bash
curl.exe -N -s -X POST http://localhost:3000/api/v1/reports/SEU_REPORT_ID/generate -H "Authorization: Bearer SEU_ACCESS_TOKEN_AQUI" -H "Content-Type: application/json" -d "{}" | grep "^data:" | sed "s/^data: //" | jq --unbuffered -rj "select(.text != null) | .text" | tee generated.md
```

O `-j` do `jq` (sem quebra de linha por saída) mais `--unbuffered` é o que dá o
efeito de texto sendo escrito ao vivo, igual ao que o `itui` vai mostrar; o
`tee` guarda o resultado em `generated.md` pra abrir num visualizador de
Markdown depois. `grep`/`sed`/`jq` vêm do Git Bash — no PowerShell puro, rode a
linha inteira dentro de `bash -c "..."`.

Para conferir só o desfecho da geração (inclusive se o texto foi cortado no
limite de tokens):

```bash
curl.exe -N -s -X POST http://localhost:3000/api/v1/reports/SEU_REPORT_ID/generate -H "Authorization: Bearer SEU_ACCESS_TOKEN_AQUI" -H "Content-Type: application/json" -d "{}" | grep -A 1 "event: done"
```

**Esperado**: `"finish_reason":"stop"`. Se vier `"length"`, o laudo terminou no
teto de `max_output_tokens` (`src/config.rs`) e está incompleto — o texto
acaba no meio de uma seção, não na última seção do documento.

Compare sempre com o modelo determinístico, que é o material que alimenta o
prompt:

```bash
curl.exe -s http://localhost:3000/api/v1/reports/SEU_REPORT_ID/draft -H "Authorization: Bearer SEU_ACCESS_TOKEN_AQUI" | jq -r .text > draft.md
```

O `-r` do `jq` é obrigatório: sem ele o `\n` do JSON vai literal pro arquivo e
o Markdown não renderiza.

---

## 7. Repetindo sob o runtime Lambda real (`cargo lambda watch`)

Os testes acima sob `cargo run` validam a lógica de negócio. Mas a
**tradução do payload** (formato de evento da API Gateway ↔ requisição
HTTP que o `axum::Router` entende) só é exercitada de verdade sob
`cargo lambda watch` — é onde o `Set-Cookie` historicamente quebra em
projetos Lambda + Rust, e é justamente o que a autenticação depende.

### 7.1. Subir o emulador

Feche o `cargo run` da seção 2.1 primeiro (mesma porta/processo trava o
próximo build — ver nota da seção 2.1), depois:

```bash
cargo lambda watch
```

Espere aparecer `starting Runtime server runtime_addr=127.0.0.1:9000`.

### 7.2. Invocar via HTTP, não via `cargo lambda invoke --data-file`

**`cargo lambda invoke --data-file evento.json` não funciona neste projeto**
— ele falha com `invalid error payload missing field 'errorType'`, porque
`main.rs` usa `lambda_http::run_with_streaming_response` (exigido pelo SSE de
`/generate`, ver CLAUDE.md) e o CLI do `cargo-lambda` não entende resposta em
modo streaming vinda desse comando. Isso não é bug do `raijin` nem indica que
a tradução de evento esteja quebrada — é limitação conhecida do `invoke`
contra runtime streaming.

O emulador também expõe uma **Function URL local**, que aceita `curl` direto
e passa pela mesma tradução evento↔HTTP do `lambda_http` — é o caminho que
funciona:

```bash
curl.exe -i -X POST http://127.0.0.1:9000/lambda-url/raijin/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"lambdawatch@teste.com\",\"password\":\"senha123456\"}"
```

**Esperado**: `HTTP/1.1 201 Created` com um header `set-cookie:
refresh_token=...; HttpOnly; SameSite=None; Secure; Path=/api/v1/auth;
Max-Age=2591999` — é o ponto crítico: confirma que o `lambda_http` traduziu o
`Set-Cookie` do Axum corretamente pra fora. Se esse header não aparecer, é
sinal de que a tradução quebrou — o problema está no formato de payload, não
no handler do Axum (que já foi validado nas seções 3-6).

### 7.3. Refresh, logout e qualquer rota autenticada

Mesma Function URL local, cookie (`-b`/`-c`) e `Authorization: Bearer` do jeito
normal:

```bash
curl.exe -i -c cookies.txt -X POST http://127.0.0.1:9000/lambda-url/raijin/api/v1/auth/refresh \
  -b cookies.txt
```

**Esperado**: `HTTP/1.1 200 OK` e um `set-cookie` novo — repita o padrão pra
`/api/v1/auth/logout`, esperando `204`. O mesmo caminho serve pra validar
qualquer rota que dependa de cookie/sessão sob o runtime real, por exemplo
`PATCH /api/v1/user/password` (troca de senha reemite sessão e revoga o
refresh token antigo — confirme com um segundo `refresh` usando o cookie
antigo salvo antes da troca, esperando `401`).

### 7.4. Testar o `lambda_source_ip` sem `X-Forwarded-For`

Este é o teste que confirma que o middleware novo (`src/main.rs`,
`lambda_source_ip`) evita o rate limiter cair em `UnableToExtractKey`
(erro 500) quando a requisição não traz `X-Forwarded-For` — situação normal
sob Function URL, onde a AWS só garante `requestContext.http.sourceIp`
(sintetizado automaticamente pelo emulador a partir da conexão TCP local).

Repita o registro da seção 7.2 com outro e-mail, sem adicionar
`X-Forwarded-For` a mão:

```bash
curl.exe -i -X POST http://127.0.0.1:9000/lambda-url/raijin/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"lambdawatch2@teste.com\",\"password\":\"senha123456\"}"
```

**Esperado**: `201` (ou `409` se reusar o e-mail — o que importa é **não**
ser `500`). Se desse `500`, o middleware não estaria sintetizando o IP
corretamente a partir do evento.

### 7.6. Encerrar

`Ctrl+C` no terminal do `cargo lambda watch`, ou:

```powershell
Get-Process cargo-lambda -ErrorAction SilentlyContinue | Stop-Process -Force
```

---

## Referência rápida — o que cada teste confirma

| Seção | O que valida |
|---|---|
| 3.1–3.2 | Registro, duplicidade de e-mail, login, mensagem genérica de erro |
| 3.3 | Por que o login Google não dá pra testar sem credenciais reais |
| 3.4 | Rotação de refresh token, janela de graça de 10s, revogação de cadeia no reuso |
| 3.5 | Logout idempotente e o `Path` explícito do cookie de remoção |
| 4 | Rate limiting montado em `/auth/*` (não é defesa de segurança per se) |
| 5 | `AuthUser` bloqueando rota protegida sem token; 404 (não 401) em rota inexistente |
| 6 | Endpoint de limpeza de sessão: auth por `X-Task-Token`, deleção real no banco |
| 6.5 | Leitura do texto gerado por IA: concatenar o SSE, conferir `finish_reason`, comparar com o `/draft` |
| 7 | Tradução de `Set-Cookie` sob o runtime Lambda real; middleware `lambda_source_ip` |
