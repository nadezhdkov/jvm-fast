<p align="center">
  <img src="https://img.shields.io/badge/Rust-stable-orange?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/Java-single--module-blue?style=for-the-badge&logo=openjdk&logoColor=white" alt="Java"/>
  <img src="https://img.shields.io/badge/status-em%20desenvolvimento-yellow?style=for-the-badge" alt="Status"/>
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=for-the-badge" alt="License"/>
  <!-- quando houver release: badge de versão do GitHub Releases (seção 15 da arquitetura) -->
  <!-- <img src="https://img.shields.io/github/v/release/nadezhdkov/jvm-fast?style=for-the-badge" alt="Release"/> -->
</p>

# jvm-fast

**Um "uv para Java" — CLI nativo em Rust para dependências, JDK e build de projetos Java single-module, sem setup.**

> `docs/architecture.md` foi escrito antes do código — é a especificação viva
> do projeto, não uma descrição do que já existe. Ver
> [`docs/architecture.md`](docs/architecture.md) para o racional completo de
> design.

---

## Índice

- [Visão Geral](#visão-geral)
- [Filosofia](#filosofia)
- [Componentes Internos](#componentes-internos)
- [Arquitetura](#arquitetura)
- [Getting Started](#getting-started)
- [Comandos Cargo](#comandos-cargo)
- [Exemplos](#exemplos)
- [Testes](#testes)
- [Status do Projeto](#status-do-projeto)
- [Versionamento](#versionamento)
- [Documentação](#documentação)
- [Contribuindo](#contribuindo)
- [Licença](#licença)

---

## Visão Geral

- Resolução e download de dependências, com lockfile determinístico
- Gerenciamento de versões de JDK via Eclipse Temurin — `jdk install`/`jdk
  list`/`jdk use`, e `[project].java-version` (incl. alias `"lts"`) resolvido
  e instalado automaticamente (com confirmação, a menos que `--yes`) por
  `install`/`update` (Fase 2 completa)
- Compilação, execução e testes diretos com `jvmfast build`/`run`/`test`
  (via `javac`/`java`, sem POM ou build Gradle intermediário) — **Fase 3
  completa**
- Testes via JUnit Platform Console Standalone, tratado como dependência
  interna (baixado e cacheado automaticamente, nunca declarado no
  manifesto)
- Cache global de artefatos, content-addressable
- Suporte a BOMs para gestão centralizada de versões
- Exclusions de dependências transitivas
- **Fora de escopo na v1**: build multi-módulo, plugins de terceiros,
  empacotamento avançado (shaded jars), publicação em repositórios, JPMS —
  ver [`docs/architecture.md#1`](docs/architecture.md) para a lista
  explícita do que **não** pertence à v1.

---

## Filosofia

- **Declaração e resolução nunca são a mesma struct** — `Module` declara o
  que um projeto precisa; só `Workspace` resolve o que efetivamente vai para
  o classpath.
- **Topologia separada de estado de resolução** — `GraphEdge` (quem trouxe o
  quê) e `ResolvedNode` (o que foi decidido, e por quê) nunca se fundem numa
  única struct.
- **Mediação de conflitos por precedência fixa**, nunca heurísticas
  concorrentes: profundidade menor vence, depois versão maior, depois
  desempate determinístico.
- **O lockfile é autossuficiente** — `jvmfast why` reconstrói qualquer
  decisão de resolução só a partir de `project.lock`, sem re-fetch.
- **O cache nunca é fonte de verdade** — corrupção é sempre resolvida
  reconstruindo, nunca com reparo em memória.

Detalhamento completo em [`docs/architecture.md`](docs/architecture.md).

---

## Componentes Internos

| Componente | Responsabilidade | Status |
|---|---|---|
| `src/domain` | Modelo de domínio da arquitetura (seção 3.1): `Module`, `Workspace`, `DependencyGraph`, `Lockfile` | Todos os tipos construídos de verdade — `Workspace`/`Lockfile` pelo marco de lockfile, `DependencyGraph`/`ResolvedNode` pela mediação |
| `src/manifest` | Parsing de `project.toml` em `Module`, com erros tipados | Implementado |
| `src/version` | Version ranges (`^`/`~`/exato) e exclusão de pré-release (seção 6.1) | Implementado, usado pela mediação para comparar versões |
| `src/bom` | Tabela `coordenada → versão` a partir de BOMs declarados (seção 3.3) | Implementado, ligado ao grafo |
| `src/exclusion` | Filtro de transitivas excluídas (seção 3.4), agregado por workspace | Implementado, ligado ao grafo |
| `src/pom` | Parser real de `pom.xml` (`quick-xml`) + `PomProvider` compartilhado + `HttpPomProvider` (fetch via layout Maven, síncrono) | Implementado — sem interpolação de propriedade nem herança de `<parent>` |
| `src/graph` | Constrói o grafo de candidatos do workspace (seção 6.2, passo 4) | Implementado, ligado à mediação |
| `src/mediation` | Decide o vencedor de cada conflito (seção 6.2, passo 5) — produz `DependencyGraph`/`ResolvedNode` | Implementado, ligado ao lockfile |
| `src/lockfile` | Hash de manifesto, `DependencyGraph` → `Lockfile`, leitura/escrita de `project.lock` (seção 4) | Implementado — `sha256`/`resolved-from` ainda vêm de fora, sem download real |
| `src/workspace` | `load_workspace` — primeiro construtor real de `Workspace` | Implementado — ainda não decide sozinho se o lock está válido |
| `src/cache` | Cache de artefatos content-addressable + índice SQLite (seção 5) | Implementado, ligado ao download |
| `src/download` | Download paralelo de artefatos via `reqwest`/`tokio` (seção 6.2 passo 6) | Implementado — primeiro código `async` do projeto |
| `src/maven` | Layout de path Maven (`group/artifact/version/...`) compartilhado entre `pom::http` e `download` | Implementado |
| `src/resolve` | Orquestra BOMs → exclusions → grafo → mediação (seção 6.2, passos 3–5) | Implementado |
| `src/cli` | Comandos `install`/`update`/`add`/`remove`/`build`/`run`/`test`/`tree`/`why`/`jdk` (seção 9) | Fases 1, 2 e 3 completas |
| `src/jdk` | Instala JDKs Temurin via API do Adoptium (seção 7) + resolve o alias `"lts"` | Implementado |
| `src/config` | Leitura/escrita de `[defaults]` em `~/.config/jvmfast/config.toml` (seção 3.5) | Implementado — só `[defaults]`; `[network]`/`[output]` ainda não são lidos |
| `src/build` | Compila `src/main/java` com `javac` + copia `src/main/resources` para `target/classes` (seção 8) | Implementado |
| `src/run` | Executa `[run].main-class` via `java`, stdio herdado (seção 8) | Implementado |
| `src/testing` | Resolve `[dev-dependencies]`, baixa o JUnit Platform Console Standalone e roda testes (seção 8.1) | Implementado — dev-deps não persistem em `project.lock` ainda |

---

## Arquitetura

```text
Module          → declara o que precisa    (project.toml)
Workspace       → resolve o que será usado (project.lock)

GraphEdge       → topologia pura (quem trouxe o quê)
ResolvedNode    → estado de resolução (o que venceu, e por quê)
```

Ver [`docs/architecture.md#3.1`](docs/architecture.md) para o modelo
completo, e a seção 6 para o fluxo de resolução ponta a ponta.

---

## Getting Started

**Requisitos**: toolchain Rust stable (via [rustup](https://rustup.rs)) e,
a partir da Fase 3, uma JDK real instalada e no `PATH` (`javac`/`java`) —
`tests/build.rs`/`tests/cli_build.rs` rodam `javac` de verdade, não um mock.

Não há release nem binário publicado ainda — para experimentar o código
atual:

```bash
git clone https://github.com/nadezhdkov/jvm-fast.git
cd jvm-fast
cargo build
cargo test
```

Com o binário compilado, a Fase 1 já funciona ponta a ponta num projeto
Java real (resolve, baixa artefatos, escreve `project.lock`):

```bash
cd /caminho/do/seu/projeto   # com um project.toml (ver Exemplos abaixo)
cargo run --manifest-path /caminho/do/jvm-fast/Cargo.toml -- install
```

```bash
cargo run --manifest-path /caminho/do/jvm-fast/Cargo.toml -- jdk install 21
cargo run --manifest-path /caminho/do/jvm-fast/Cargo.toml -- jdk list
cargo run --manifest-path /caminho/do/jvm-fast/Cargo.toml -- jdk use 21
```

`install`/`update` já resolvem `[project].java-version` (auto-instalando a
JDK do projeto, com confirmação a menos que `--yes` seja passado);
`jvmfast build` compila `src/main/java`/copia `src/main/resources` para
`target/classes` usando essa JDK; `jvmfast run` compila e executa
`[run].main-class`; e `jvmfast test` compila `src/test/java` e roda os
testes via JUnit Platform Console Standalone — a Fase 3 inteira já
funciona ponta a ponta.

---

## Comandos Cargo

| Comando | Descrição |
|---|---|
| `cargo build` | Compila o binário `jvmfast` |
| `cargo test` | Roda todos os testes |
| `cargo test <nome>` | Roda um teste específico por substring do nome |
| `cargo clippy --all-targets -- -D warnings` | Lint — CI falha em qualquer warning |
| `cargo fmt --all` / `-- --check` | Formata / verifica formatação sem escrever |

---

## Exemplos

`project.toml` mínimo:

```toml
[project]
name = "meu-projeto"
version = "0.1.0"
java-version = "21"

[dependencies]
"com.fasterxml.jackson.core:jackson-databind" = "2.17.0"
"org.slf4j:slf4j-api" = "2.0.13"

[dev-dependencies]
"org.hamcrest:hamcrest" = "2.2"

[repositories]
default = "https://repo1.maven.org/maven2"

[run]
main-class = "com.exemplo.Main"
```

```bash
jvmfast install                                    # resolve e baixa dependências, gera project.lock
jvmfast install --yes                               # idem, mas instala a JDK do projeto sem perguntar (CI)
jvmfast add "com.fasterxml.jackson.core:jackson-databind@2.17.0"
jvmfast remove "org.slf4j:slf4j-api"
jvmfast update                                      # re-resolve ignorando o lock existente
jvmfast tree                                        # árvore de dependências resolvida
jvmfast why "com.fasterxml.jackson.core:jackson-core"
jvmfast build                                       # compila src/main/java para target/classes
jvmfast run                                         # compila e executa [run].main-class
jvmfast test                                        # compila src/test/java e roda via JUnit Platform Console
jvmfast test --filter "tag:fast"                    # só testes com essa tag
jvmfast test --filter "*.UserTest"                  # só classes cujo nome bate com o glob
jvmfast test --report-xml                           # grava relatórios JUnit XML em target/test-reports
```

---

## Testes

- Testes usam fixtures locais (manifestos/POMs sintéticos) — nunca rede
  real, mesmo para o resolvedor (ver
  [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md)).
- Nomes de teste descrevem o comportamento esperado, não a issue/PR que os
  motivou.
- Exceção: `tests/build.rs`/`tests/cli_build.rs`/`tests/run.rs`/
  `tests/cli_run.rs`/`tests/cli_test.rs` (Fase 3) rodam contra a JDK real
  do ambiente (`javac`/`java` no `PATH`) — `jvmfast build`/`run`/`test` só
  invocam um compilador/JVM de verdade, então não faz sentido mocká-los.
  `tests/cli_test.rs` também baixa o JUnit Platform Console Standalone do
  Maven Central real de propósito (é a dependência interna que
  `jvmfast test` sempre baixa de lá, nunca do repositório do projeto) — a
  segunda e única outra exceção deste repo à regra de nunca tocar rede
  real em teste.

```bash
cargo test
```

---

## Status do Projeto

Roadmap por fases (ver [`docs/architecture.md#12`](docs/architecture.md)
para detalhes; estado detalhado dos marcos em
[`CLAUDE.md`](CLAUDE.md#roadmap--whats-implemented-vs-next)):

- [x] Bootstrap do projeto Rust — modelo de domínio + parsing de `project.toml`
- [x] Marco — version ranges `^`/`~` (seção 6.1)
- [x] Marco — resolução de BOMs (seção 3.3)
- [x] Marco — exclusions (seção 3.4)
- [x] Marco — grafo de transitivas + fetch real de POM (`quick-xml`)
- [x] Marco — mediação de conflitos (seção 6.2) — `DependencyGraph`/`ResolvedNode` reais
- [x] Marco — lockfile read/write + manifest-hash (seção 4) — primeiro `Workspace` real
- [x] Marco — cache content-addressable + índice SQLite (seção 5)
- [x] Marco — download paralelo via reqwest/tokio + `HttpPomProvider` (seção 6.2)
- [x] Marco — comandos CLI: `install`/`add`/`remove`/`update`/`tree`/`why`
- [x] **Fase 1 completa** — resolução e cache, ponta a ponta
- [x] Marco — `jvmfast jdk install`/`jdk list`/`jdk use` via API do Adoptium (seção 7)
- [x] Marco — resolução de `java-version` no manifesto (incl. alias `"lts"`) em `install`/`update`, persistida em `project.lock`
- [x] **Fase 2 completa** — gerenciamento de JDK, ponta a ponta
- [x] Marco — `jvmfast build`: compila `src/main/java` com `javac` + copia `src/main/resources` (seção 8)
- [x] Marco — `jvmfast run`: compila e executa `[run].main-class`/`jvm-args` via `java` (seção 8)
- [x] Marco — `jvmfast test`: `[dev-dependencies]` + JUnit Platform Console Standalone (seção 8.1)
- [x] **Fase 3 completa** — build/run/test, ponta a ponta
- [x] Marco — `jvmfast import-pom`: gera `project.toml` a partir de um `pom.xml` existente (seção 10)
- [ ] Marco — `jvmfast import-gradle` via Gradle Tooling API (seção 10)
- [ ] **Fase 4 completa** — interoperabilidade, ponta a ponta
- [ ] Fase 5 — workspace e multi-módulo

---

## Versionamento

Ainda não há release publicado, então não há `CHANGELOG.md` ainda. Quando
houver: SemVer para o binário (`jvmfast --version`), desacoplado da versão
do formato do lockfile (`project.lock`, campo `version`) — ver
[`docs/architecture.md#15.4`](docs/architecture.md).

---

## Documentação

- [`docs/architecture.md`](docs/architecture.md) — a especificação completa
  de arquitetura (fonte de verdade para decisões de design; em português)
- [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) — convenções de código e
  commit para quem for contribuir
- [`CLAUDE.md`](CLAUDE.md) — estado atual do código e roadmap de próximos
  marcos
- [`STYLE.md`](STYLE.md) — guia de estilo para prosa e mensagens
  voltadas ao usuário (CLI e docs); em inglês, parte dele descreve
  comportamento ainda não implementado (ver ressalva no próprio arquivo)

---

## Contribuindo

Erros de domínio são tipados (`thiserror`, nunca `anyhow`/`String`
genérico); `async` só onde há concorrência real; `rustfmt`/`clippy` rodam
sem `#[allow(...)]` não explicado. Ver
[`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) para a lista completa, ou
[`CONTRIBUTING.md`](CONTRIBUTING.md) (em inglês) para o guia completo de
setup, testes, lint e processo de contribuição. Vulnerabilidades de
segurança seguem [`SECURITY.md`](SECURITY.md).

---

## Licença

Dual-licenciado sob [MIT](LICENSE-MIT) ou [Apache 2.0](LICENSE-APACHE), à
sua escolha.
