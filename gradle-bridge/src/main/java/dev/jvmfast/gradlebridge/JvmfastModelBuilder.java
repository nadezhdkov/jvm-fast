package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import org.gradle.api.Project;
import org.gradle.tooling.provider.model.ToolingModelBuilder;

/**
 * Registered with the target build's
 * {@link org.gradle.tooling.provider.model.ToolingModelBuilderRegistry} by
 * {@link JvmfastModelBuilderPlugin} (docs/architecture.md seção 10, step
 * 1). {@code buildAll} walking {@code project.getConfigurations()} to
 * populate a real {@link JvmfastDependencyModel} — the actual dependency
 * resolution — is the next Fase 4 milestone, not this one; see CLAUDE.md's
 * "Known, deliberate gaps inside Fase 4" for why it's deferred rather than
 * stubbed with fabricated data.
 */
public class JvmfastModelBuilder implements ToolingModelBuilder {

    @Override
    public boolean canBuild(String modelName) {
        return JvmfastDependencyModel.class.getName().equals(modelName);
    }

    @Override
    public Object buildAll(String modelName, Project project) {
        throw new UnsupportedOperationException(
                "JvmfastModelBuilder.buildAll is not implemented yet — see CLAUDE.md's Fase 4 gaps");
    }
}
