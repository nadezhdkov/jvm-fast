# Convenções do projeto jvm-fast

Este documento cobre convenções práticas de código e contribuição. Decisões
de arquitetura, design do resolvedor, formato de manifesto/lockfile etc.
vivem em [`architecture.md`](architecture.md) — este arquivo
não repete esse conteúdo, só referencia.

## Nomenclatura

- **`jvm-fast`** (com hífen) — nome do projeto: repositório, documentação,
  identidade. Usado em prosa e títulos.
- **`jvmfast`** (sem hífen) — nome do binário que o usuário invoca. Usado
  apenas em exemplos de comando (`jvmfast install`, `jvmfast run`).
- Nunca misturar as duas formas dentro do mesmo contexto (ex.: não escrever
  "o jvmfast é uma ferramenta..." em prosa, nem "rode `jvm-fast install`" em
  exemplo de comando).

## Terminologia interna

Os termos abaixo têm significado técnico específico neste projeto (seção
3.1 da arquitetura) — usar com o sentido exato, não como sinônimos livres:

| Termo | Significa |
|---|---|
| `Module` | Declara dependências (`project.toml`). Nunca resolve sozinho. |
| `Workspace` | Resolve o que efetivamente vai para o classpath (`project.lock`). Ponto de entrada único da resolução. |
| `Dependency` | Intenção declarada (`VersionReq`), não versão resolvida. |
| `ResolvedNode` | Estado de resolução de um artefato no grafo — não tem noção de topologia. |
| `GraphEdge` | Topologia pura (quem trouxe o quê) — não tem estado de resolução. |

Regra de ouro: **declaração e resolução nunca se misturam na mesma
estrutura**. Se uma mudança de código faz um `Module` carregar uma versão
resolvida, ou faz um `ResolvedNode` carregar informação de aresta, é sinal
de que a mudança está violando a separação da seção 3.1 e precisa ser
repensada, não só ajustada.

## Código Rust

- `rustfmt` e `clippy` rodam sem exceções silenciadas (`#[allow(...)]` exige
  comentário explicando o motivo pontual, não uso genérico para calar o
  linter)
- Erros de domínio (falha de resolução, autenticação, rede — seção 11 da
  arquitetura) são tipados por categoria própria, nunca `anyhow`/`String`
  genérico no caminho principal — os exit codes da seção 11 dependem de
  distinguir a categoria do erro em tempo de compilação, não de inspecionar
  mensagem
- Funções que tocam rede ou filesystem são `async` (`tokio`) só quando
  precisam de concorrência real (downloads paralelos, seção 6.2); operação
  pontual e sequencial (parse de manifesto, escrita de lockfile) fica
  síncrona — não colocar `async` por hábito
- Structs que representam formato persistido (`project.toml`,
  `project.lock`) derivam `Serialize`/`Deserialize` diretamente sobre os
  tipos de domínio quando possível; evitar um tipo "DTO" espelhado
  artificialmente só para satisfazer serde, a menos que o formato externo
  divirja de fato do modelo interno

## Testes

- Testes do resolvedor usam fixtures locais (POMs sintéticos, servidor HTTP
  mock) — nunca dependem de Maven Central real (seção 13.1 da arquitetura).
  Isso vale para qualquer teste automatizado, não só os listados na seção
  13.1: rede real em teste é motivo de rejeição em review
- Um caso de teste novo para o resolvedor de conflitos deve nomear o cenário
  pelo comportamento esperado (`depth_wins_over_higher_version`,
  `same_depth_ties_break_by_version`), não pelo número da issue ou PR que o
  motivou

## Commits

- Mensagens descrevem o *porquê*, não só o *o quê* — o diff já mostra o
  que mudou
- Mudança de comportamento observável pelo usuário (novo comando, nova
  flag, mudança de formato de output) menciona isso explicitamente na
  mensagem, mesmo que o PR também inclua refactor interno
- Mudança que afeta o formato de `project.toml` ou `project.lock` referencia
  a seção correspondente da arquitetura (ex. "seção 3.3") na mensagem —
  facilita auditar depois se a decisão documentada ainda bate com o código

## Documentação

- `architecture.md` é a fonte de verdade de design — mudança de
  comportamento que contradiz o que está lá exige atualizar o documento no
  mesmo PR, não depois
- Este `CONVENTIONS.md` é sobre *como* escrever código/commits; não é lugar
  para registrar decisões de design de produto — essas vão para a
  arquitetura

## Template de README (crates internas)

Aplica-se ao README de uma crate Rust do projeto, **se e quando** o código
for organizado como workspace multi-crate — a arquitetura (`architecture.md`,
seção 2) não compromete o projeto a essa divisão hoje, só lista as crates
*externas* usadas (toml, reqwest, clap etc.). Este template só passa a valer
a partir do momento em que uma decisão de dividir o binário em crates
internas (`jvmfast-core`, `jvmfast-cli` etc.) for tomada e registrada na
arquitetura; até lá, não há README de crate para escrever. Não é para o
README raiz do projeto, que segue seu próprio formato de apresentação.
Adaptado do template de dependência única do Kotlin/Gradle original para
`Cargo.toml`, já que o código Rust do jvm-fast usa Cargo como gerenciador de
build.

**Não cobre componentes não-Rust.** O helper JVM `jvmfast-gradle-bridge.jar`
(seção 10 da arquitetura) é Java, não uma crate — não tem `Cargo.toml`, não
é instalado como dependência de `[dependencies]`, e seu README precisaria de
uma variante própria do template (instalação via Maven/Gradle coordinate ou
apenas via jar bundlado, não via Cargo). Não usar este template para ele
sem adaptar a seção de Instalação.

````markdown
# jvmfast-{{crate}}

**{{uma linha descrevendo o propósito da crate}}**

<p align="left">
  <img src="https://img.shields.io/badge/status-{{planejado|em%20desenvolvimento|est%C3%A1vel}}-{{lightgrey|yellow|brightgreen}}?style=flat-square" alt="Status"/>
  <img src="https://img.shields.io/badge/depende%20de-{{jvmfast--core}}-blue?style=flat-square" alt="Dependências"/>
</p>

---

## Índice

- [O que resolve](#o-que-resolve)
- [O que não resolve](#o-que-não-resolve)
- [Instalação](#instalação)
- [Exemplo Rápido](#exemplo-rápido)
- [API Principal](#api-principal)
- [Quando usar (e quando não usar)](#quando-usar-e-quando-não-usar)
- [Notas de Design](#notas-de-design)
- [Testes](#testes)
- [Changelog](#changelog)

---

## O que resolve

{{ex.: jvmfast-core implementa o resolvedor de dependências (Workspace/
Module/DependencyGraph, seção 3.1 da arquitetura) — resolução de versões,
mediação de conflitos, sem nenhuma dependência de I/O de rede direta.}}

## O que não resolve

{{deixar explícito o limite — evita o problema de escopo confuso já citado
como risco na seção 13 da arquitetura. Ex.: "não faz download de artefatos
— isso é responsabilidade de jvmfast-net; esta crate só decide o que
precisa ser baixado."}}

---

## Instalação

```toml
[dependencies]
jvmfast-{{crate}} = "<versão>"
```

## Exemplo Rápido

{{snippet mínimo mostrando o caso de uso mais comum da crate}}

## API Principal

{{tipos e funções públicas centrais — não é referência exaustiva
(isso é o que `cargo doc` já gera), só o que orienta por onde começar}}

## Quando usar (e quando não usar)

{{casos de uso legítimos vs. casos em que outra crate do workspace é a
escolha certa — evita a crate virar dumping ground de responsabilidades
não relacionadas}}

## Notas de Design

{{decisões não óbvias específicas desta crate; decisões de arquitetura
mais amplas ficam em `architecture.md`, linkadas por seção, não
duplicadas aqui}}

## Testes

{{como rodar os testes desta crate isoladamente, e que tipo de fixture
ela usa — ver convenção de testes acima}}

## Changelog

{{link para o changelog da crate, se mantido separado do changelog raiz}}
````

Regras de preenchimento:
- **Status** reflete a fase do roadmap (seção 12 da arquitetura) em que a
  crate está — nunca "estável" antes de ter suíte de testes cobrindo os
  casos da seção 13.1 (quando aplicável ao escopo da crate)
- **"O que não resolve"** é obrigatório, não opcional — mesmo quando parece
  óbvio. É a seção que evita a crate crescer por acúmulo de responsabilidade
  não planejada
- README de crate nunca duplica prosa de racional de design já coberta na
  arquitetura — linka por seção (`seção 3.1`, `seção 6.2`) em vez de
  reexplicar
