# Taxonomia de Não Conformidades — Achados Fotográficos

Cinco categorias de não conformidade elétrica, com exemplo de foto rotulada e parágrafo de análise técnica redigido por engenheiro. Fonte: apêndice fotográfico de `relatorio/assets/templatecomdatas.docx` (Figuras 9–13, 23 fotos JPEG reais) e capítulo de resultados de `relatorio/assets/modelos de relatórios.pdf`. Esse material **não existe em nenhum outro lugar do repositório** — o formulário atual (`riscos`, `equipamentos` etc.) não tem uma taxonomia de achados, só de riscos/EPIs pré-inspeção.

> Nenhum dos dois arquivos-fonte deve ser portado como template — ambos estão superados pelo `template.docx` ativo, que já não tem esse apêndice fotográfico. Este documento resgata só o **conteúdo de domínio** (a taxonomia e o registro linguístico), não o arquivo.

## Para que serve

1. **Categorizar as fotos anexadas pelo usuário.** Hoje o upload de imagem (`report_images`) não tem categoria/tag nenhuma — é só uma lista de arquivos. As 5 categorias abaixo são candidatas naturais a um campo `finding_category` opcional por imagem.
2. **Few-shot para a integração com Groq/Llama-3.** O CLAUDE.md já prevê IA auxiliando a geração do laudo (§1, §2). Estes 5 parágrafos mostram exatamente o registro linguístico, o nível de detalhe técnico e a estrutura (causa → risco → consequência operacional) que um parecer gerado por IA deveria imitar.
3. **Modelo de diagramação da seção de imagens.** O template ativo hoje só empilha fotos sem legenda nem agrupamento (ver anti-padrão em `domain-glossary.md` §6). O apêndice órfão mostra o formato desejável: grade de fotos rotuladas `(a)(b)(c)…` + legenda numerada + parágrafo de análise.

---

## 1. Condutores energizados expostos e sem proteção

**Exemplos fotografados**: derivação de circuito em aberto, descontinuidade da linha, emenda fora de caixa de derivação, condutores vivos em contato com o solo, emendas sem isolação.

**Parecer técnico (exemplo, corrigido de erros de digitação do original — "v6ulneráveis"→"vulneráveis", "ocosionar"→"ocasionar"):**

> "A ocorrência desta prática cria o risco de choques elétricos pois os pontos destacados encontram-se na área externa onde o pessoal de jardinagem poderia inadvertidamente tocar ou seccionar tais fios com uso de pás, enxadas etc. A ausência de caminhos adequados para os condutores aliado às más práticas de manutenção podem ocasionar a interrupção do fornecimento de energia nos circuitos envolvidos e um tempo de solução bastante elevado para o reconhecimento do ponto do problema."

**Estrutura do parecer**: risco imediato à pessoa (mecanismo de exposição) → causa raiz (má prática de manutenção) → consequência operacional (interrupção de fornecimento, dificuldade de diagnóstico).

## 2. Aterramentos improvisados

**Exemplos fotografados**: condutores de aterramento vulneráveis mecanicamente, hastes de aterramento fora de caixa de inspeção.

**Estrutura do parecer**: exposição a dano mecânico/intempérie → comprometimento da eficácia do aterramento → risco de choque em caso de falta.

## 3. Condições das emendas

**Exemplos fotografados**: conector perfurante em derivação de alimentador, emendas somente com fita isolante de baixa tensão, excesso de emendas na mesma linha.

**Trecho de análise (excesso de emendas)**:

> "...o excesso de emendas aumenta a impedância dos condutores contribuindo para a queda de tensão na linha e propicia o surgimento de pontos quentes."

Nota: é a **única** menção a um fenômeno de queda de tensão em todo o material-fonte, e é qualitativa — sem fórmula, sem valor limite. Não contradiz o achado de que não há cálculo de engenharia no sistema (ver `nbr-5410-tests.md`).

## 4. Linhas elétricas mal instaladas ou afixadas

**Exemplos fotografados**: ausência de fixação adequada, linhas vulneráveis mecanicamente, localização inapropriada de proteção e das linhas.

**Estrutura do parecer**: falha de fixação/roteamento → vulnerabilidade a dano mecânico → risco de rompimento/exposição futura.

## 5. Sinais de ocorrência de curtos ou pontos quentes

**Exemplos fotografados**: curto nos terminais de disjuntor por má conexão e presença de elementos externos, perda de isolação dos condutores por sobrecorrente e má conexão ("perca da isolação" no original — corrigido para "perda"), ponto de curto-circuito em barramento.

**Estrutura do parecer**: evidência física observada (queima, descoloração) → causa técnica provável (má conexão, sobrecorrente) → risco residual (recorrência, incêndio).

---

## Padrão de diagramação (do apêndice órfão, não do template ativo)

```
Figura N. <Tema geral>: (a) <descrição curta> (b) <descrição curta> (c) <descrição curta> ...
```

- Grade de 2 a 6 fotos por figura, dispostas lado a lado, rotuladas `(a)`, `(b)`, `(c)`...
- Legenda única abaixo da grade, numerando e descrevendo cada foto em uma frase curta.
- Um parágrafo de análise técnica logo após a legenda, seguindo a estrutura causa → risco → consequência descrita acima.

Isso contrasta com o `template.docx` **ativo**, que hoje só imprime as fotos em sequência (`{%p for foto in imagens %}`), sem legenda, sem numeração, sem agrupamento temático e com largura fixa de 50 mm. A nova stack, gerando o documento no client-side, tem liberdade para recuperar esse formato mais estruturado — é melhoria de produto, não migração 1:1, mas o padrão de referência já existe e não precisa ser inventado do zero.
