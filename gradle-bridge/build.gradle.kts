// jvmfast-gradle-bridge — the one non-Rust component in the jvm-fast stack
// (docs/architecture.md seção 10, "Migração de projetos Gradle"). This is a
// standalone Gradle project on purpose, with its own build/versioning,
// separate from the Cargo build that produces the `jvmfast` binary — see
// CLAUDE.md's Fase 4 writeup for how the two are meant to fit together.
plugins {
    java
    id("com.gradleup.shadow") version "9.6.1"
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
    // org.gradle:gradle-tooling-api stopped publishing new versions to
    // Maven Central a while back (its metadata there tops out at an old
    // 7.x snapshot) — current releases (matching the 9.6.1 wrapper this
    // project pins) live on Gradle's own repository instead.
    maven("https://repo.gradle.org/gradle/libs-releases")
}

dependencies {
    // The plugin/model-builder classes (org.gradle.api.Plugin,
    // org.gradle.tooling.provider.model.*) only ever run inside a real
    // Gradle build's own classloader, loaded via the init-script
    // jvmfast generates (seção 10) — the Gradle API is provided by that
    // runtime, never bundled into this jar, hence compileOnly.
    compileOnly(gradleApi())
    testImplementation(gradleApi())
    testImplementation(gradleTestKit())

    // Main.java (the Tooling API *client*-side driver, invoked as a plain
    // `java -jar` process — never inside any Gradle build's own
    // classloader) needs its own copy of the Tooling API, shaded into the
    // jar by shadowJar below. This never collides with the compileOnly
    // gradleApi() above: the two run in entirely separate JVM invocations
    // (target build vs. this bridge's own client process), never on the
    // same classpath at once.
    implementation("org.gradle:gradle-tooling-api:9.6.1")
    // The Tooling API logs via SLF4J; nop avoids an "no SLF4J providers
    // were found" warning on every invocation without pulling in a real
    // logging backend jvmfast has no use for.
    runtimeOnly("org.slf4j:slf4j-nop:2.0.16")

    testImplementation(platform("org.junit:junit-bom:5.10.2"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    // `ProjectBuilder` (used by `JvmfastModelBuilderTest`) injects synthetic
    // classes into its classloader via reflection at project-creation time —
    // blocked by JDK 17+'s module system without this, failing with
    // `IllegalAccessException: module java.base does not open java.lang to
    // unnamed module`. A known, documented requirement for Gradle's own test
    // fixtures on JDK 17+, not specific to this project.
    jvmArgs("--add-opens", "java.base/java.lang=ALL-UNNAMED")
}

// Produces the single jar `build.rs` embeds into the `jvmfast` binary
// (seção 10: "um helper JVM empacotado com o jvmfast") — a shaded/fat jar
// because Main.java's Tooling API client code needs `gradle-tooling-api`
// (and its own transitive deps) actually present on the classpath when
// invoked as `java -jar`, unlike the plugin/model classes above which rely
// on the target build's own Gradle API instead. Classified `-all` (rather
// than reusing the plain `jar` task's output name) so the two never
// collide on disk when both run in the same `gradle-bridge` checkout.
tasks.shadowJar {
    archiveClassifier.set("all")
    manifest {
        attributes["Main-Class"] = "dev.jvmfast.gradlebridge.Main"
    }
}
