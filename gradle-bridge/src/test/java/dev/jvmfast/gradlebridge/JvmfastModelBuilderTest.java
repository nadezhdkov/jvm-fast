package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
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
    void buildAllIsNotImplementedYet() {
        assertThrows(
                UnsupportedOperationException.class,
                () -> builder.buildAll(JvmfastDependencyModel.class.getName(), null));
    }
}
