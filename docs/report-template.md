# Modelo do relatório — extraído do `template.docx` legado

Transcrição literal do texto fixo de `relatorio/assets/template.docx` (`word/document.xml`) do
repositório `gerador`, para servir de fonte única ao modelo determinístico de `src/document/`.
Consulta pontual e somente leitura, conforme CLAUDE.md; nenhum outro conteúdo do `gerador` foi lido.

**Achado desta transcrição, e por isso não há um `## Prosa fixa` neste documento**: o template não
tem texto narrativo. Nenhuma frase de abertura, nenhuma ligação entre seções, nenhum parecer de
encerramento — é o formulário despejado direto em tabelas do Word, com placeholder `{{ campo }}` no
lugar de cada resposta. Confirma o que `domain-glossary.md` (linha 247) já registrava: o documento
legado "era só o miolo de tabelas, recortado de um documento maior". Capa, cabeçalho institucional,
ART e parecer conclusivo não existem para portar — são desenho novo.

O que existe de fato, e que este arquivo fixa como fonte:

## Ordem e títulos das seções (verbatim)

1. `Avaliação e planejamento da execução`
2. `Avaliação das influencias externas da Instalação elétrica [5410]`
3. `Avaliação qualitativa da instalação elétrica`
4. `Avaliação quantitativa da Instalação [8]`
5. `Imagens do Relatório`

Essa é a ordem canônica que `src/document/sections.rs` reproduz.

## Cabeçalho do laudo (antes da seção 1)

Campos fixos no topo do documento, fora de qualquer tabela — `{{ data }}`, `{{ hora }}`,
`{{ local }}`, `{{ temperatura }}`, `{{ clima }}`, `{{ responsaveis }}`:

```
Data da inspeção: {{ data }} Hora da inspeção: {{ hora }} h
Local: {{ local }}   Condições climáticas: Temperatura:{{ temperatura }}°C   Clima {{ clima }}
Responsáveis: {{ responsaveis }}
```

`local` e `responsaveis` mapeiam para `location_code` e `responsible_parties` — **não entram** no
modelo determinístico gerado por `src/document/`, pelo mesmo motivo que não entram no prompt da IA
(ver CLAUDE.md, regra de privacidade do `location_code`, estendida aqui a `responsible_parties`
por ser identificação de pessoa real). Cabeçalho com esses dados é montado pelo `itui`, que já os
tem via `GET /reports/{id}`.

## Cabeçalhos de tabela (verbatim, por seção)

**Seção 1** (`inspection_planning`): `Item | Descrição | Detalhamento | Observação`. Os 17 itens
numerados e seus enunciados já estão em `domain-glossary.md` §2 — não repetidos aqui.

**Seção 2** (`external_influences`): `Item | Descrição | Classificação | Tipo | Item da norma NBR
5410`. As classificações (códigos AA–CB) e a cláusula por item já estão em `nbr-5410-choices.json`
(`nbrClause`) — não repetidos aqui.

**Seção 3** (`qualitative_assessment`): duas linhas de cabeçalho —
```
ITEM | DESCRIÇÃO DO ITEM | ASPECTOS OBSERVADOS ATENDEM A NORMA? | Item da Norma NBR 5410
      |                    | (S) SIM   (N) NÃO   (P) PARCIALMENTE | OBSERVAÇÕES
```
Confirma o ternário S/N/P já registrado em `domain-glossary.md` §4. Os 23 itens e a cláusula por
item já estão lá — não repetidos aqui. O item 10 (espaço-reserva) traz a tabela normativa embutida
no próprio template:
```
Qtde de Circuitos | Espaço reserva
Até 6              | 2
7 a 12             | 3
13 a 30            | 4
N > 30             | 0,15 N
```
Já capturada com mais precisão (arredondamento) em `domain::required_spare_circuits` — a versão do
template é a redação de origem, não a fonte de cálculo.

**Seção 4** (`quantitative_assessment`):
- Parte I — cabeçalho `Quadro Distribuição – Alimentador principal | Circuitos terminais`, colunas
  de circuito `Circuito | Fase | Disjuntor | Descrição | Condutor | Corrente`. **Confirma o teto de
  13 circuitos do legado**: linhas fixas `circuito0`…`circuito12`, sem loop — a regra "sem limite de
  circuitos" do CLAUDE.md segue valendo, esta é só a evidência primária.
- Parte II — cabeçalho `ITEM DA NORMA | DESCRIÇÃO DO ENSAIO | ASPECTOS OBSERVADOS` /
  `(S) SIM  (N) NÃO | MOTIVO | OBSERVAÇÕES`. Procedimento e critério de cada ensaio já estão em
  `nbr-5410-tests.md` — não repetidos aqui.

**Seção 5** (`imagens`): sem cabeçalho de tabela — um loop raso `{%p for foto in imagens %}`,
imagem após imagem, sem legenda nem agrupamento. É o anti-padrão que `findings-taxonomy.md` já
documenta e que o modelo novo substitui pelo padrão do apêndice órfão (grade `(a)(b)(c)` + legenda
numerada + parágrafo de análise).

## Consequência para `src/document/template.rs`

Como não há prosa fixa a transcrever, o modelo determinístico não é "preencher os espaços de um
texto pronto" — é **montar** o documento a partir da estrutura acima: título de seção (verbatim,
§"Ordem e títulos"), lista de campo→resposta em pt-BR (rótulos de `domain-glossary.md`, valores de
`nbr-5410-choices.json`), tabela de circuitos sem teto, e o apêndice de imagens no padrão de
`findings-taxonomy.md`. Prosa fixa mínima de transição entre seções pode ser escrita agora, mas é
composição nova — não recuperação do legado — e deve ser tratada como tal no código (comentário
apontando aqui, não alegando ser o texto original).
