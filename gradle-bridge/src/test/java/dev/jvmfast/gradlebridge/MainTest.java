package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.DefaultJvmfastDependency;
import dev.jvmfast.gradlebridge.model.DefaultJvmfastDependencyModel;
import dev.jvmfast.gradlebridge.model.DefaultJvmfastModule;
import dev.jvmfast.gradlebridge.model.JvmfastDependency;
import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import dev.jvmfast.gradlebridge.model.JvmfastModule;
import org.junit.jupiter.api.Test;

import java.util.Collections;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

class MainTest {

    @Test
    void toJsonProducesTheShapeTheRustSideParses() {
        JvmfastDependency dependency =
                new DefaultJvmfastDependency("com.example:demo", "1.0.0", "compileClasspath");
        JvmfastModule module =
                new DefaultJvmfastModule("demo", "0.1.0", List.of(dependency));
        JvmfastDependencyModel model = new DefaultJvmfastDependencyModel(List.of(module));

        String json = Main.toJson(model);

        assertEquals(
                "{\"modules\":[{\"name\":\"demo\",\"version\":\"0.1.0\",\"dependencies\":"
                        + "[{\"coordinate\":\"com.example:demo\",\"version\":\"1.0.0\","
                        + "\"configuration\":\"compileClasspath\"}]}]}",
                json);
    }

    @Test
    void toJsonHandlesAModuleWithNoDependencies() {
        JvmfastModule module = new DefaultJvmfastModule("demo", "0.1.0", Collections.emptyList());
        JvmfastDependencyModel model = new DefaultJvmfastDependencyModel(List.of(module));

        assertEquals(
                "{\"modules\":[{\"name\":\"demo\",\"version\":\"0.1.0\",\"dependencies\":[]}]}",
                Main.toJson(model));
    }

    @Test
    void toJsonEscapesQuotesAndBackslashesInStringFields() {
        JvmfastDependency dependency =
                new DefaultJvmfastDependency("a\"b\\c", "1.0", "compileClasspath");
        JvmfastModule module = new DefaultJvmfastModule("n", "v", List.of(dependency));

        String json = Main.toJson(new DefaultJvmfastDependencyModel(List.of(module)));

        assertTrue(json.contains("\"coordinate\":\"a\\\"b\\\\c\""));
    }
}
