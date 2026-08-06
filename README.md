# jvm-fast

**Um "uv para Java" — CLI nativo em Rust para dependências, JDK e build de projetos Java single-module, sem setup.**

<p align="center">
  <img src="https://img.shields.io/badge/Rust-stable-orange?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/Java-single--module-blue?style=for-the-badge&logo=openjdk&logoColor=white" alt="Java"/>
  <img src="https://img.shields.io/badge/status-em%20desenvolvimento-yellow?style=for-the-badge" alt="Status"/>
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=for-the-badge" alt="License"/>
  <!-- quando houver release: badge de versão do GitHub Releases (seção 15 da arquitetura) -->
  <!-- <img src="https://img.shields.io/github/v/release/nadezhdkov/jvm-fast?style=for-the-badge" alt="Release"/> -->
</p>

[![rust](https://github.com/nadezhdkov/jvm-fast/actions/workflows/rust.yml/badge.svg)](https://github.com/nadezhdkov/jvm-fast/actions/workflows/rust.yml)
[![docs](https://github.com/nadezhdkov/jvm-fast/actions/workflows/docs.yml/badge.svg)](https://github.com/nadezhdkov/jvm-fast/actions/workflows/docs.yml)

---

## Status: bootstrap inicial, em desenvolvimento ativo

Este projeto **ainda não tem release nem CLI funcional**. O que existe hoje:
parsing de `project.toml` em memória e o modelo de domínio da arquitetura
(`Module`/`Workspace`/`DependencyGraph`). Resolução de dependências,
lockfile, cache, download e os comandos (`install`, `add`, `run`, etc.)
ainda **não estão implementados** — ver o roadmap em
[`CLAUDE.md`](CLAUDE.md#roadmap--whats-implemented-vs-next).

Não há binário para instalar ainda. Para experimentar o código atual:

```bash
git clone https://github.com/nadezhdkov/jvm-fast.git
cd jvm-fast
cargo build
cargo test
```

## O que é

`jvm-fast` mira o nicho de scripts, projetos single-module e prototipagem
rápida em Java — resolução e download de dependências, gerenciamento de
versões de JDK, e compilação/execução direta, sem exigir POM ou build
Gradle intermediário. Não substitui Maven/Gradle em builds multi-módulo
complexos; o objetivo é velocidade de resolução e ausência de setup, com
caminho de migração para Maven/Gradle quando o projeto crescer.

Quando implementado, o fluxo de trabalho é este `project.toml`:

```toml
[project]
name = "meu-projeto"
version = "0.1.0"
java-version = "21"

[dependencies]
"com.fasterxml.jackson.core:jackson-databind" = "2.17.0"
"org.slf4j:slf4j-api" = "2.0.13"

[dev-dependencies]
"org.junit.jupiter:junit-jupiter" = "5.10.2"

[run]
main-class = "com.exemplo.Main"
```

```bash
jvmfast install   # resolve e baixa dependências
jvmfast run       # compila e executa
jvmfast test      # roda testes via JUnit Platform Console
```

## Documentação

- [`docs/architecture.md`](docs/architecture.md) — a especificação completa
  de arquitetura (fonte de verdade para decisões de design; em português)
- [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) — convenções de código e
  commit para quem for contribuir
- [`CLAUDE.md`](CLAUDE.md) — estado atual do código e roadmap de próximos
  marcos

## Licença

Dual-licenciado sob [MIT](LICENSE-MIT) ou [Apache 2.0](LICENSE-APACHE), à
sua escolha.
