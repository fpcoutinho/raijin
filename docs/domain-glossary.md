# Glossário de Domínio — Laudo de Engenharia Elétrica

Mapa canônico entre os nomes do **Django legado** e os nomes em **inglês** da nova stack.

> **Fonte única de nomenclatura.** Antes de nomear qualquer campo, coluna, struct, prop ou rota relacionada ao laudo, consulte esta tabela. Não invente nomes novos: se algo estiver faltando, adicione aqui primeiro.

## Como ler

- **Legado** — nome do campo em `relatorio/models.py` do repositório `gerador` (congelado).
- **Novo** — nome canônico em `snake_case` (database e Rust). No frontend, converta para `camelCase` mecanicamente: `professional_qualification` → `professionalQualification`. Componentes React em `PascalCase`.
- **Rótulo (pt-BR)** — texto exibido ao usuário. Fica em arquivo de i18n/constantes no frontend, **nunca** hardcoded em componente.
- **Tipo** — tipo pretendido na nova stack, **não** o tipo do legado (que é `CharField(255)` para quase tudo).

Listas de opções (choices) não estão aqui — estão em [`nbr-5410-choices.json`](nbr-5410-choices.json), em formato consumível por código.

---

## Entidades

| Legado | Novo | Observação |
|---|---|---|
| `Relatorio` | `Report` | Tabela única de ~90 colunas no legado. **Reavalie a modelagem** antes de copiar essa forma. |
| `Circuito` | `Circuit` | Uma linha da tabela do quadro de distribuição. |
| `Imagens` | `ReportImage` | Nome legado está no plural incorretamente. |
| `rel_pai` (FK) | `report_id` | |
| `autor` (FK User) | `author_id` | |

---

## 1. Dados principais (`report`)

| Legado | Novo | Rótulo (pt-BR) | Tipo |
|---|---|---|---|
| `data` | `inspected_at` | Data e Hora da inspeção | `timestamptz` |
| `local` | `location_code` | Local da inspeção | `text` |
| `temperatura` | `ambient_temperature_c` | Temperatura (em °C) | `int` |
| `clima` | `weather_conditions` | Condições Climáticas | `text` |
| `responsaveis` | `responsible_parties` | Responsáveis | `text[]` |
| — (novo) | `status` | Situação do laudo | `report_status` (enum) |

**Regras:**
- `location_code` segue o padrão `BLOCO-SALA`, validado pelo regex `[A-Z]{2,}-[A-Z]{0,}[0-9]{2,}` (ex.: `CCHLA-102`, `CI-T02`). O prefixo antes do `-` é o **bloco**, usado no auto-preenchimento (ver §2).
- `responsible_parties` é texto livre separado por vírgula no legado. Modele como array de verdade.
- Atenção: `ambient_temperature_c`/`weather_conditions` (medição pontual da inspeção) são **campos distintos** de `ambient_temperature_class`/`climatic_conditions_class` da §3 (classificação NBR).
- **`status`** substitui o "wizard implícito" do legado (que inferia a etapa concluída checando se um campo-sentinela estava vazio — ver `CLAUDE.md`). Enum Postgres `report_status`: `draft` / `in_review` / `approved` / `archived` (ajustar valores conforme o fluxo real do `raijin`).

---

## 2. Planejamento e segurança (`inspection_planning`)

Ao criar um laudo, se o autor já possui outro laudo no mesmo **bloco**, todos os campos desta seção são copiados do laudo anterior ([relatorio/views.py:56](../relatorio/views.py:56)).

| Legado | Novo | Rótulo (pt-BR) | Tipo |
|---|---|---|---|
| `qualiprof` | `professional_qualification` | Qual a qualificação profissional dos responsáveis pela inspeção? | enum |
| `integridade` | `team_fit_for_work` | Os participantes da inspeção estão bem fisicamente e mentalmente? | bool |
| `dialogo` | `safety_briefing_held` | Houve diálogo de segurança? | bool |
| `curso_nr` | `has_nr10_training` | Um ou mais executores da inspeção possui curso NR-10? | bool |
| `conferido` | `service_pre_checked` | O serviço foi preliminarmente conferido? | bool |
| `riscos` | `identified_hazards` | Quais riscos foram detectados? | enum[] |
| `equipamentos` | `safety_equipment` | Quais equipamentos de segurança serão utilizados? | enum[] |
| `desligamento` | `requires_shutdown` | Este serviço requer desligamento ou bloqueio de equipamento ou rede? | bool |
| `sinalizacao` | `signage_used` | Este serviço requer sinalização? | enum[] |
| `delimitar_area` | `requires_area_delimitation` | Necessita delimitar a área de trabalho? | bool |
| `auxconces` | `requires_utility_assistance` | Necessita de auxílio de concessionária local? | bool |
| `tensao` | `requires_voltage_check` | Necessário fazer verificação de tensão? | bool |
| `aterramento` | `requires_temporary_grounding` | A inspeção requer aterramento temporário? | bool |
| `altura` | `work_at_height` | A inspeção será realizada em altura? | bool |
| `cinto_seg` | `requires_safety_harness` | Será necessário se aprisionar à escada e utilização de cinto de segurança? | bool |
| `requi_seg` | `safety_requirements_met` | Os requisitos de segurança foram atendidos por todos? | bool |
| `reavaliacao` | `requires_reassessment` | Houve necessidade de reavaliação das inspeções realizadas? | bool |

**Nota:** os campos `bool` são `"Sim"`/`"Não"` em `CharField(max_length=4)` no legado. Use booleano real. Os `enum[]` são listas Python serializadas como string (`"['Queda', 'Choque']"`) e lidas com `ast.literal_eval` — normalize.

---

## 3. Influências externas (`external_influences`) — NBR 5410

Todos os campos desta seção são enums de códigos normativos. **Opções em [`nbr-5410-choices.json`](nbr-5410-choices.json) — transcreva literalmente, não reescreva.** Cada item também tem uma cláusula NBR 5410 (`nbrClause` no JSON) extraída do texto fixo do `template.docx` — ausente em `forms.py`, e marcada como "a verificar" (ver Pendências).

| Legado | Novo | Grupo NBR | Rótulo (pt-BR) |
|---|---|---|---|
| `tempambiente` | `ambient_temperature_class` | AA | Temperatura Ambiente |
| `condambiente` | `climatic_conditions_class` | AB | Condições climáticas do ambiente |
| `altitude` | `altitude_class` | AC | Altitude |
| `presagua` | `water_presence_class` | AD | Presença de água |
| `pressolidos` | `solid_bodies_presence_class` | AE | Presença de corpos sólidos |
| `pressubst` | `corrosive_substances_class` | AF | Presença de substâncias corrosivas ou poluentes |
| `solmecanicas` | `mechanical_impact_class` | AG | Impactos mecânicos |
| `solmecanicas` | `vibration_class` | AH | Vibrações |
| `presmofo` | `flora_and_mold_class` | AK | Presença de flora e mofo |
| `presfauna` | `fauna_presence_class` | AL | Presença de fauna |
| `infleletro` | `electromagnetic_influence_class` | AM | Influências eletromagnéticas, eletrostáticas ou ionizantes |
| `radsolar` | `solar_radiation_class` | AN | Radiação solar |
| `descatm` | `lightning_exposure_class` | AQ | Descargas atmosféricas |
| `movdoar` | `air_movement_class` | AR | Movimentação do ar |
| `vento` | `wind_class` | AS | Vento |
| `competencia` | `people_competence_class` | BA | Competência das pessoas |
| `reseletr` | `body_electrical_resistance_class` | BB | Resistência elétrica do corpo humano no ambiente |
| `contpessoas` | `earth_potential_contact_class` | BC | Contato das pessoas com o potencial da terra |
| `condfuga` | `evacuation_conditions_class` | BD | Condições de fuga das pessoas em emergências |
| `natmatpr` | `processed_materials_class` | BE | Natureza dos materiais processados ou armazenados |
| `natmatcons` | `construction_materials_class` | CA | Qual a natureza dos materiais de construção |
| `classestr` | `building_structure_class` | CB | Qual a classificação da estrutura das edificações |

**Nota de modelagem:** `mechanical_impact_class` (AG) e `vibration_class` (AH) eram um único campo de escolha exclusiva no legado (`solmecanicas`). São grupos independentes na norma — um ambiente pode ter impacto leve (AG1) e vibração severa (AH3) ao mesmo tempo (ex.: compressores próximos). Modele como dois campos separados.

### ⚠️ Erros de transcrição no legado

Estes são **bugs de dados** no `forms.py` atual. **Não os replique** — corrija consultando o texto oficial da NBR 5410 antes de transcrever:

1. **`presfauna` (AL)** — os valores são `AL1`/`AL2`, mas os rótulos exibidos dizem `AK1`/`AK2` (copiados de `presmofo`).
2. **`condambiente` (AB)** — `AB2` e `AB3` têm texto idêntico ("Ambientes internos e externos com temperaturas baixas").
3. **`infleletro` (AM)** — `AM3-1` e `AM3-2` têm texto idêntico ("Variação de amplitude da tensão nível controlado"); `AM3-2` deveria ser outro nível.

---

## 4. Avaliação qualitativa (`qualitative_assessment`)

**Confirmado contra `template.docx`**: a resposta é **ternária** — `Sim`/`Não`/`Parcialmente` (S/N/P no cabeçalho da tabela do Word) — não booleana. No legado é uma string única `"Sim: observação aqui"` separada por `.split(': ')`, o que quebra a exportação inteira (`IndexError`) se o valor não contiver `': '`. **Modele como objeto real** `{ answer, notes }` (`answer`: enum `yes`/`no`/`partial`; `notes`: texto livre).

Cada item tem uma cláusula NBR 5410 própria, extraída do texto fixo do template (campo `nbrClauses` em [`nbr-5410-choices.json`](nbr-5410-choices.json)) — ausente em `forms.py`. Onde marcado `null`, o template não trazia referência legível.

| Legado | Novo | Rótulo (pt-BR) | Tipo |
|---|---|---|---|
| `documentacao` | `has_installation_documentation` | Há documentação da instalação e esta inclui plantas, esquemas unifilares e outros, detalhes de montagem, memorial descritivo, especificações de componentes, parâmetros de projeto? | answer+notes |
| `ambientesofreu` | `renovation_documentation_updated` | O ambiente sofreu alguma reforma e a documentação foi atualizada ou acrescida de algum aditivo de projeto? | answer+notes |
| `instalacaoinspecionada` | `inspected_before_commissioning` | A instalação foi inspecionada antes da entrada em funcionamento e existe algum documento atestando esse fato? | answer+notes |
| `linhaseletricasdisp` | `wiring_allows_maintenance_access` | As linhas elétricas estão dispostas de modo a permitir verificações, ensaios, reparos ou modificação da instalação? | answer+notes |
| `compinstalacao` | `components_selected_for_external_influences` | Os componentes da instalação foram selecionados e instalados levando-se em conta as influências externas? | answer+notes |
| `linhaseletricascorr` | `wiring_correctly_installed` | As linhas elétricas estão corretamente instaladas? | answer+notes |
| `tomadasdeforca` | `outlets_comply_nbr14136` | As tomadas de força existentes atendem ao novo padrão nacional NBR 14136/2002? | answer+notes |
| `qtdesufitomadas` | `sufficient_outlet_count` | O ambiente apresenta tomadas de força em quantidade suficiente? | answer+notes |
| `instlquadist` | `distribution_board_accessible` | O quadro de distribuição está devidamente instalado em local de fácil acesso à manutenção, inspeção e ensaio? | answer+notes |
| `novoscircuitos` | `spare_circuit_capacity` | Há disponibilidade de criação de novos circuitos no quadro de distribuição? | enum + campo derivado (ver [`nbr-5410-tests.md`](nbr-5410-tests.md)) |
| `advquadist` | `distribution_board_warning_labels` | Há indicações de advertência nos quadros de distribuição? | answer+notes |
| `dispprotecaoident` | `protection_devices_identified` | Os dispositivos de proteção estão dispostos e identificados de forma fácil de reconhecer os respectivos circuitos protegidos? | answer+notes |
| `protcircuitos` | `protection_matches_conductor_gauge` | A proteção dos circuitos é compatível com a bitola dos condutores? | answer+notes |
| `barramentoquadist` | `has_neutral_and_earth_busbars` | O Quadro de distribuição possui barramento de neutro e aterramento? | answer+notes |
| `bitola` | `terminals_match_conductor_gauge` | Todas as conexões estão com terminais apropriados para cada bitola utilizada? | answer+notes |
| `condutident` | `conductors_color_identified` | Os condutores estão identificados por cores ou conforme sua função? | answer+notes |
| `disjundif` | `has_residual_current_device` | Existe disjuntor diferencial residual instalado no quadro de distribuição? | answer+notes |
| `dispprotecaosurtos` | `has_surge_protection_device` | Existe dispositivo de proteção contra surtos de tensões? | answer+notes |
| `servseguranca` | `has_safety_service_equipment` | Há elementos para serviços de segurança a exemplo de iluminação de emergência, exaustores de fumaça, etc? | answer+notes |
| `esqaterramento` | `earthing_system_type` | Qual o esquema de aterramento utilizado? | enum — determina o ramo condicional do ensaio 7.3.5, ver [`nbr-5410-tests.md`](nbr-5410-tests.md) |
| `reservadeenergia` | `has_backup_power_source` | Existe fonte alternativa ou de reserva de energia? | answer+notes |
| `fontseguranca` | `has_safety_power_source` | Existe fonte de segurança de energia? | answer+notes |
| `paralelismo` | `has_source_paralleling_prevention` | Há mecanismos para evitar o paralelismo das fontes? | answer+notes |

---

## 5. Avaliação quantitativa (`quantitative_assessment`)

### Parte I — Quadro de distribuição / alimentador principal

| Legado | Novo | Rótulo (pt-BR) | Unidade | Tipo |
|---|---|---|---|---|
| `capbarramento` | `busbar_capacity_amps` | Capacidade de barramento | A | `numeric` |
| `protgeral` | `main_breaker_rating_amps` | Proteção Geral Disjuntor | A | `numeric` |
| `protdr` | `rcd_rating_amps` | Proteção DR | A | `numeric` |
| `protdps` | `spd_rating_amps` | Proteção DPS | A | `numeric` |
| `vab` | `voltage_ab_volts` | Vab | V | `numeric` |
| `van` | `voltage_an_volts` | Van | V | `numeric` |
| `ia` | `current_phase_a_amps` | Ia | A | `numeric` |
| `vbc` | `voltage_bc_volts` | Vbc | V | `numeric` |
| `vbn` | `voltage_bn_volts` | Vbn | V | `numeric` |
| `ib` | `current_phase_b_amps` | Ib | A | `numeric` |
| `vca` | `voltage_ca_volts` | Vca | V | `numeric` |
| `vcn` | `voltage_cn_volts` | Vcn | V | `numeric` |
| `ic` | `current_phase_c_amps` | Ic | A | `numeric` |

Sistema trifásico: `ab`/`bc`/`ca` são tensões de linha (fase-fase); `an`/`bn`/`cn` são de fase (fase-neutro). **Decidido: decimal**, não inteiro — no legado todos eram `PositiveSmallIntegerField`, mas medições reais raramente são inteiras. No Postgres, `numeric` (não `float`) para não perder precisão em valor de medição.

### Parte II — Ensaios realizados

Todos são pares **resposta + observação (`answer`/`notes`)**, com respostas `Sim`/`Não` (sem `Parcialmente` — confirmado, essa seção não é ternária como a §4). Procedimento de medição, critério numérico de aceitação e cláusula NBR 5410 de cada ensaio estão em [`nbr-5410-tests.md`](nbr-5410-tests.md) — esse conteúdo existe só no texto fixo do `template.docx`, não em `forms.py`.

| Legado | Novo | Rótulo (pt-BR) | Cláusula NBR |
|---|---|---|---|
| `continuidade` | `continuity_test` | Continuidade dos condutores de proteção e das eqüipotencializações principal e suplementar? | 7.3.2 |
| `resistencia` | `insulation_resistance_test` | Resistência de isolamento da instalação elétrica? | 7.3.3 |
| `selvpelv` | `selv_pelv_separation_test` | Resistência de isolamento aplicável a SELV, PELV e separação elétrica? | 7.3.4 |
| `verificacao` | `equipotential_bonding_test` | Verificação das condições de proteção por eqüipotencialização e seccionamento automático da alimentação? | 7.3.5 (condicional ao esquema de aterramento) |
| `ensaiodetensao` | `applied_voltage_test` | Ensaio de tensão aplicada? | 7.3.6 |
| `ensaiodefunc` | `functional_test` | Ensaio de funcionamento? | 7.3.7 |

### Parte III — Circuitos (`circuits`)

Entidade separada, N por laudo.

| Legado | Novo | Rótulo (pt-BR) |
|---|---|---|
| `modelo` | `circuit_id` | Circuito |
| `fase` | `phase` | Fase |
| `disjuntor` | `breaker` | Disjuntor |
| `descricao` | `description` | Descrição |
| `condutor` | `conductor` | Condutor |
| `corrente` | `current` | Corrente |

No legado todos são texto livre, inclusive `corrente` — sem validação numérica nem de unidade. Considere tipar de verdade. O campo chama-se `modelo` no banco mas o rótulo exibido é "Circuito" — nomeação confusa do legado, não replicar.

**⚠️ Limite rígido de 13 circuitos no legado.** O `template.docx` não itera essa tabela — são linhas fixas `circuito0`…`circuito12` no Word. O código injeta quantas chaves houver via loop, mas **do 14º circuito em diante os dados são descartados silenciosamente**, sem erro nem aviso. Instalações com quadros grandes perdem circuitos no documento final sem que ninguém perceba. Na nova stack, a tabela de circuitos deve iterar de verdade (sem teto), e a geração client-side do documento precisa fazer o mesmo — não herdar esse limite.

---

## 6. Imagens (`report_images`)

| Legado | Novo | Observação |
|---|---|---|
| `img` (`CloudinaryField`) | `storage_path` | Cloudinary → Cloudflare R2 (S3-compatible; MinIO localmente em dev). |

No legado, o upload é feito por um `<input name="imagens[]" multiple>` manual, fora do ModelForm, e repetido em **todas** as views de etapa.

---

## Modelagem do banco: relacional + JSONB por seção

**Decidido.** Nem ~90 colunas planas nem um blob JSONB único — modelo híbrido:

- **Colunas relacionais nativas**: tudo que precisa de índice, busca, ordenação, FK ou é auditoria/identidade (`id`, `location_code`, `status`, `author_id`, `created_at`, `updated_at`, etc. — ver §1 Entidades).
- **Um bloco JSONB por seção temática do laudo**, correspondendo às seções deste glossário:
  - `inspection_planning jsonb` — §2.
  - `external_influences jsonb` — §3.
  - `qualitative_assessment jsonb` — §4.
  - `quantitative_assessment jsonb` — §5 (Partes I e II; `circuits` continua como tabela relacional própria, é 1:N).
  - `document_content jsonb` — estado nativo da árvore do editor TipTap.

**Por que:** no Rust, cada seção vira uma struct `serde`, convertida nativamente para JSONB via `sqlx::types::Json<T>`. Ganha-se type safety no código sem exigir `ALTER TABLE` a cada ajuste de campo secundário — relevante porque essas seções tendem a evoluir (a norma muda, o profissional pede um campo novo) com mais frequência que a identidade do laudo.

As listas de opções normativas continuam **só** em [`nbr-5410-choices.json`](nbr-5410-choices.json) — não criar tabela de referência no Postgres para isso (`ref_influencias_externas` ou similar). Essas listas mudam apenas quando a norma muda, o que já exigiria revisão de código; uma tabela de banco para isso adiciona complexidade de runtime sem ganho real, e duplica a fonte da verdade que o JSON já cobre.

## Pendências de decisão

Nenhuma pendência bloqueante no momento. `report_status`/precisão numérica/idioma do documento foram fechados — ver "Resolvido" abaixo. Novas pendências de modelagem (se surgirem durante o desenvolvimento do `raijin`) entram aqui.

## Resolvido nesta rodada

- **Estado de progresso**: adicionado campo `status` (`report_status` enum: `draft`/`in_review`/`approved`/`archived`) — ver §1 e Entidades. Substitui o wizard implícito do legado.
- **Unidades e precisão**: medições da §5 Parte I viram `numeric` (decimal), não `int`. Ver §5.
- **Tradução do laudo gerado**: confirmado — os nomes deste glossário são de código; o documento final continua 100% em português.

- **Espaço de reserva no quadro de distribuição** (`spare_circuit_capacity`): a NBR 5410 6.5.4.7 define uma tabela de espaço-reserva por faixa de circuitos que o legado descartava (guardava só a faixa escolhida, sem calcular a saída). Ver [`nbr-5410-tests.md`](nbr-5410-tests.md) — a nova stack deve implementar como campo derivado.
- **Tipo de resposta da avaliação qualitativa**: confirmado ternário (S/N/P), não booleano — ver §4.
- **Cláusulas NBR 5410 e redação de AB/AM**: **decidido seguir a fonte original sem verificação adicional contra o texto oficial da norma.** As cláusulas (`nbrClause`) usam a numeração sequencial extraída de `template.docx`; os textos de `climatic_conditions_class` (AB) e `electromagnetic_influence_class` (AM) — incluindo as duplicações conhecidas (AB2/AB3, AM3-1/AM3-2) — são transcritos como estão na fonte, sem tentativa de reescrever ou "corrigir" contra uma definição alternativa. Isso fecha a pendência anterior; não reabrir sem uma cópia confiável da norma em mãos.
- **Limite de 13 circuitos**: **decidido remover.** É limitação do `template.docx` do legado (linhas fixas), não uma regra de negócio. A nova stack itera livremente, sem teto.
- **Diagramação da seção de imagens**: **decidido adotar o padrão do apêndice órfão** (`templatecomdatas.docx`) como modelo — grade de fotos rotuladas `(a)(b)(c)…`, legenda numerada, parágrafo de análise técnica por grupo. Ver [`findings-taxonomy.md`](findings-taxonomy.md) para a taxonomia e o tom de referência. Substitui o comportamento pobre do template ativo (fotos soltas, sem legenda, sem agrupamento).
- **Elementos de responsabilidade técnica no documento**: **decidido incluir** capa, cabeçalho institucional, campo de ART e assinatura/parecer conclusivo — nenhum existe no legado (era só o miolo de tabelas, recortado de um documento maior). É **funcionalidade nova**, a ser desenhada do zero no `itui`/no gerador de documento client-side, não uma migração do template do `gerador`.
