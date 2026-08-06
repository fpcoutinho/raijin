# Ensaios da Avaliação Quantitativa — NBR 5410

Procedimento, critério de aceitação e cláusula normativa dos 6 ensaios da §5 (Parte II) do [`domain-glossary.md`](domain-glossary.md). Esse conteúdo existe **apenas** em `relatorio/assets/template.docx` (texto fixo da tabela) e em `relatorio/assets/modelos de relatórios.pdf` — o formulário atual (`forms.py`) descarta tudo isso e reduz cada ensaio a um `Sim`/`Não` sem contexto.

> **Fonte**: extraído por leitura direta do `.docx` (`word/document.xml`) e do PDF de referência. Transcrição literal do texto normativo — não reescrever.

## Como usar

Cada ensaio abaixo tem: a pergunta (rótulo já usado no glossário), a cláusula da NBR 5410, o procedimento de medição, e o critério numérico de aceitação quando existir. Na nova stack, isso deve virar texto de apoio (tooltip/ajuda) ao lado do campo `answer` de `quantitative_assessment`, e — onde houver valor numérico de referência — um campo de medição real com validação contra o limite, não apenas Sim/Não.

---

### 1. Continuidade dos condutores de proteção (`continuity_test`)

- **Cláusula NBR 5410**: 7.3.2
- **Pergunta**: Continuidade dos condutores de proteção e das eqüipotencializações principal e suplementar?
- **Procedimento**: *"Aplicar fonte de tensão à vazio de 4 a 24 VAC ou VDD com corrente mínima de 0,2 A"*
- **Critério de aceitação**: não numérico no material-fonte (verificação de continuidade elétrica, não de valor de resistência).

### 2. Resistência de isolamento da instalação (`insulation_resistance_test`)

- **Cláusula NBR 5410**: 7.3.3
- **Pergunta**: Resistência de isolamento da instalação elétrica?
- **Procedimento**: *"Medir a resistência entre os condutores vivos tomados dois a dois sem a presença de equipamentos de utilização e entre cada condutor vivo e o terra"*
- **Critério de aceitação**: *"Para circuitos com tensão nominal até 500 V usar uma tensão de ensaio de 500 Vdd e obter **R ≥ 0,5 MΩ**"*

### 3. Resistência de isolamento SELV/PELV (`selv_pelv_separation_test`)

- **Cláusula NBR 5410**: 7.3.4
- **Pergunta**: Resistência de isolamento aplicável a SELV, PELV e separação elétrica?
- **Procedimento**: *"A medição deve ser efetuada preferencialmente com os equipamentos de utilização conectados"*
- **Critério de aceitação**: *"Para circuitos com extra baixa tensão funcional e SELV usar uma tensão de ensaio de 250 Vdd e obter **R ≥ 0,25 MΩ**"*

### 4. Verificação das condições de proteção (`equipotential_bonding_test`)

- **Cláusula NBR 5410**: 7.3.5
- **Pergunta**: Verificação das condições de proteção por eqüipotencialização e seccionamento automático da alimentação?
- **Procedimento é condicional ao esquema de aterramento** já coletado em `earthing_system_type` (§4 do glossário). Ramifique o formulário/relatório conforme o valor desse campo:

  - **Se TN**: *"medir a impedância do percurso da corrente de falta (A medição da impedância do percurso da corrente de falta, num esquema IT, requer o curto-circuitamento temporário do ponto neutro da alimentação com o condutor de proteção.)"* / *"verificação das características do dispositivo de proteção associado, e no caso de DR fazer ensaio"*
  - **Se TT**: *"medição da resistência de aterramento das massas da instalação (realizada com corrente alternada) Quando for inviável a medição da resistência de aterramento pode ser substituída pela medição da impedância (ou resistência) do percurso da corrente de falta"* / *"inspeção visual e ensaio dos dispositivos DR."*
  - **Se IT**: *"verificação da corrente de primeira falta por cálculo ou medição"* / *"verificação das condições de proteção em caso de dupla falta"*

  Nota: o texto entre parênteses do ramo TN parece trocado no original (fala de esquema IT dentro do ramo TN) — preservado literalmente da fonte; considerar checagem contra o texto oficial da norma antes de usar como texto de ajuda ao usuário.

### 5. Ensaio de tensão aplicada (`applied_voltage_test`)

- **Cláusula NBR 5410**: 7.3.6
- **Pergunta**: Ensaio de tensão aplicada?
- **Procedimento**: *"A tensão de ensaio deve ser aplicada durante 1 min."* / *"Durante o ensaio não devem ocorrer arcos nem disrupções"* / *"Consultar tabela 61 da NBR 5410 para a tensão a ser aplicada."*
- **Critério de aceitação**: ausência de arco/disrupção durante 1 minuto de aplicação; o valor de tensão a aplicar depende da Tabela 61 da norma (não reproduzida nas fontes lidas).

### 6. Ensaio de funcionamento (`functional_test`)

- **Cláusula NBR 5410**: 7.3.7
- **Pergunta**: Ensaio de funcionamento?
- **Procedimento**: *"Verificar se quadros elétricos, acionamentos, controles, intertravamentos, comandos etc, se encontram corretamente montados, ajustados e instalados. Os dispositivos de proteção devem ser submetidos a ensaios de funcionamento, se necessário, para verificar se estão corretamente instalados e ajustados."*
- **Critério de aceitação**: qualitativo (inspeção visual/funcional), sem valor numérico.

---

## Regra tabelada: espaço de reserva no quadro de distribuição

Única regra **computável** de todo o domínio (o resto é avaliação qualitativa por inspeção). Cláusula NBR 5410 **6.5.4.7**, referenciada em `spare_circuit_capacity` (item 10 da avaliação qualitativa, §4 do glossário):

| Quantidade de circuitos | Espaço de reserva exigido |
|---|---|
| Até 6 | 2 |
| 7 a 12 | 3 |
| 13 a 30 | 4 |
| N > 30 | 0,15 × N |

O legado (`forms.py:142`, campo `novoscircuitos`) transformou isso em `ChoiceField` com as faixas como rótulo (incluindo uma opção `"Nenhuma"` que não existe na tabela normativa) e **descartou a coluna de saída** — o engenheiro escolhe a faixa, mas o sistema nunca calcula quanto espaço é exigido.

**Recomendação para a nova stack**: campo derivado, não escolha inerte. Entrada = número de circuitos (inteiro); saída = espaço de reserva calculado pela tabela acima (`reserve = lookup(n)`, com `ceil(0.15 * n)` para N > 30). Isso substitui a `ChoiceField` de faixas por um cálculo real — é funcionalidade nova mínima, não migração 1:1, mas é a única "conta" de todo o domínio e é trivial de implementar.

## O que continua sendo avaliação puramente qualitativa

Fora a regra de espaço de reserva acima, **confirma-se que não existe nenhum outro cálculo de engenharia no domínio** — sem dimensionamento de condutor, sem queda de tensão, sem ampacidade, sem seleção de disjuntor por corrente. A verificação disjuntor×condutor (`protection_matches_conductor_gauge`, item 13 da avaliação qualitativa) é **julgamento do profissional**, não fórmula: a Parte I da avaliação quantitativa coleta `breaker`/`conductor`/`current` por circuito, mas nada confronta esses valores automaticamente — nem no legado, nem nas fontes originais lidas. Se a nova stack quiser essa verificação automática, é funcionalidade nova a projetar do zero, não um cálculo recuperável da fonte.
