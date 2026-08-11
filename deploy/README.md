# deploy/

`aws-deploy.md` — passo a passo do deploy em produção: o que criar na AWS, em que
ordem, com os valores de configuração. Comece por aí.

`itui-checklist.md` — o que configurar do lado do frontend, para levar ao outro
repositório.

`deploy.ps1` — deploy do dia a dia: compila para ARM64 e atualiza o código da função.

`eventbridge-cleanup-sessions-payload.json` — payload estático do alvo do
AWS EventBridge Scheduler para `POST /tasks/cleanup-sessions`. O Scheduler
invoca a Lambda diretamente (`lambda:InvokeFunction`), sem passar por API
Gateway/Function URL, então o evento precisa ser forjado no formato APIGW v2
que `lambda_http` espera — é isso que este arquivo é.

Ao configurar o Scheduler, cole este JSON como "Input" do alvo, substituindo
`REPLACE_WITH_TASK_TOKEN_VALUE` pelo valor real de `TASK_TOKEN` (o mesmo
configurado na Lambda). Não commitar o valor real aqui.
