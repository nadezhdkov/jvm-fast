package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.JvmfastDependency;
import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import dev.jvmfast.gradlebridge.model.JvmfastModule;
import org.gradle.api.Project;
import org.gradle.testfixtures.ProjectBuilder;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

class JvmfastModelBuilderTest {

    private final JvmfastModelBuilder builder = new JvmfastModelBuilder();

    @Test
    void canBuildRecognizesTheJvmfastDependencyModel() {
        assertTrue(builder.canBuild(JvmfastDependencyModel.class.getName()));
    }

    @Test
    void canBuildRejectsAnyOtherModelName() {
        assertFalse(builder.canBuild("org.gradle.tooling.model.eclipse.EclipseProject"));
    }

    @Test
    void buildAllReturnsOneEmptyModuleForAProjectWithoutTheJavaPlugin() {
        Project project = ProjectBuilder.builder().build();

        JvmfastDependencyModel model =
                (JvmfastDependencyModel) builder.buildAll(JvmfastDependencyModel.class.getName(), project);

        assertEquals(1, model.getModules().size());
        assertTrue(model.getModules().get(0).getDependencies().isEmpty());
    }

    /**
     * Exercises real dependency resolution against real Maven Central —
     * same deliberate, narrow exception to "tests never touch the
     * network" that `tests/cli_test.rs` (Rust side, Fase 3) already makes
     * for the JUnit Console Standalone download; here it's unavoidable
     * because `buildAll`'s entire job is walking a *resolved* Gradle
     * configuration, and this project's own test suite already needs
     * network to resolve its JUnit dependencies to run at all.
     */
    @Test
    void buildAllResolvesRealDependenciesPerConfiguration() {
        Project project = ProjectBuilder.builder().build();
        project.getPluginManager().apply("java");
        project.getRepositories().mavenCentral();
        project.getDependencies().add("implementation", "org.slf4j:slf4j-api:2.0.16");
        project.getDependencies().add("testImplementation", "junit:junit:4.13.2");

        JvmfastDependencyModel model =
                (JvmfastDependencyModel) builder.buildAll(JvmfastDependencyModel.class.getName(), project);

        JvmfastModule module = model.getModules().get(0);
        assertTrue(module.getDependencies().stream().anyMatch(this::isSlf4jApiOnCompileClasspath));
        assertTrue(module.getDependencies().stream().anyMatch(this::isJunitOnTestCompileClasspath));
    }

    private boolean isSlf4jApiOnCompileClasspath(JvmfastDependency dependency) {
        return dependency.getCoordinate().equals("org.slf4j:slf4j-api")
                && dependency.getVersion().equals("2.0.16")
                && dependency.getConfiguration().equals("compileClasspath");
    }

    private boolean isJunitOnTestCompileClasspath(JvmfastDependency dependency) {
        return dependency.getCoordinate().equals("junit:junit")
                && dependency.getConfiguration().equals("testCompileClasspath");
    }
}
