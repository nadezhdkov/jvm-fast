package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import org.gradle.tooling.provider.model.ToolingModelBuilder;
import org.gradle.tooling.provider.model.ToolingModelBuilderRegistry;
import org.gradle.tooling.provider.model.UnknownModelException;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertTrue;

class JvmfastModelBuilderPluginTest {

    @Test
    void applyRegistersExactlyOneJvmfastModelBuilder() {
        RecordingRegistry registry = new RecordingRegistry();
        JvmfastModelBuilderPlugin plugin = new JvmfastModelBuilderPlugin(registry);

        // The plugin never dereferences `project` — it only forwards the
        // injected registry — so a null Project is a legitimate stand-in
        // here rather than pulling in a full Gradle test-project harness
        // for a skeleton this small.
        plugin.apply(null);

        assertEquals(1, registry.registered.size());
        ToolingModelBuilder registered = registry.registered.get(0);
        assertInstanceOf(JvmfastModelBuilder.class, registered);
        assertTrue(registered.canBuild(JvmfastDependencyModel.class.getName()));
    }

    private static final class RecordingRegistry implements ToolingModelBuilderRegistry {
        final List<ToolingModelBuilder> registered = new ArrayList<>();

        @Override
        public void register(ToolingModelBuilder builder) {
            registered.add(builder);
        }

        @Override
        public ToolingModelBuilder getBuilder(String modelName) throws UnknownModelException {
            throw new UnsupportedOperationException("not used by this test");
        }
    }
}
