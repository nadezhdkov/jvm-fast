// jvmfast-gradle-bridge — the one non-Rust component in the jvm-fast stack
// (docs/architecture.md seção 10, "Migração de projetos Gradle"). This is a
// standalone Gradle project on purpose, with its own build/versioning,
// separate from the Cargo build that produces the `jvmfast` binary — see
// CLAUDE.md's Fase 4 writeup for how the two are meant to fit together.
plugins {
    java
}

group = "dev.jvmfast"
version = "0.1.0"

java {
    // No toolchain block on purpose — auto-provisioning a JDK needs
    // network access to a toolchain repository, which isn't guaranteed
    // for every contributor/CI environment. This just targets bytecode
    // compatible with whatever JDK runs the build instead.
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

repositories {
    mavenCentral()
}

dependencies {
    // The plugin/model-builder classes (org.gradle.api.Plugin,
    // org.gradle.tooling.provider.model.*) only ever run inside a real
    // Gradle build's own classloader, loaded via the init-script
    // jvmfast generates (seção 10) — the Gradle API is provided by that
    // runtime, never bundled into this jar, hence compileOnly.
    compileOnly(gradleApi())
    testImplementation(gradleApi())

    testImplementation(platform("org.junit:junit-bom:5.10.2"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}
