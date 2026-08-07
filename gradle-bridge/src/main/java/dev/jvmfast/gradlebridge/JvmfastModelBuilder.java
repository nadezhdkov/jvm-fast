package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.DefaultJvmfastDependency;
import dev.jvmfast.gradlebridge.model.DefaultJvmfastDependencyModel;
import dev.jvmfast.gradlebridge.model.DefaultJvmfastModule;
import dev.jvmfast.gradlebridge.model.JvmfastDependency;
import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import dev.jvmfast.gradlebridge.model.JvmfastModule;
import org.gradle.api.Project;
import org.gradle.api.artifacts.Configuration;
import org.gradle.api.artifacts.ModuleVersionIdentifier;
import org.gradle.api.artifacts.result.ResolutionResult;
import org.gradle.api.artifacts.result.ResolvedComponentResult;
import org.gradle.tooling.provider.model.ToolingModelBuilder;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Registered with the target build's
 * {@link org.gradle.tooling.provider.model.ToolingModelBuilderRegistry} by
 * {@link JvmfastModelBuilderPlugin} (docs/architecture.md seção 10, step
 * 1).
 */
public class JvmfastModelBuilder implements ToolingModelBuilder {

    /**
     * The three resolvable configurations a plain `java`/`java-library`
     * project always exposes (docs/architecture.md seção 10: "cada um com
     * suas dependências resolvidas por configuração") — production
     * compile-time, production runtime, and test compile-time. A
     * configuration absent from a given project (no java plugin applied,
     * or a build that renamed/removed it) is silently skipped, never an
     * error — `buildAll` always returns a valid model, possibly with zero
     * dependencies.
     */
    private static final String[] CONFIGURATIONS = {
        "compileClasspath", "runtimeClasspath", "testCompileClasspath"
    };

    @Override
    public boolean canBuild(String modelName) {
        return JvmfastDependencyModel.class.getName().equals(modelName);
    }

    @Override
    public Object buildAll(String modelName, Project project) {
        List<JvmfastDependency> dependencies = new ArrayList<>();
        for (String configurationName : CONFIGURATIONS) {
            Configuration configuration = project.getConfigurations().findByName(configurationName);
            if (configuration == null || !configuration.isCanBeResolved()) {
                continue;
            }
            collectResolvedDependencies(configuration, configurationName, dependencies);
        }

        JvmfastModule module = new DefaultJvmfastModule(
                project.getName(), String.valueOf(project.getVersion()), dependencies);
        return new DefaultJvmfastDependencyModel(Collections.singletonList(module));
    }

    /**
     * Reads the fully resolved dependency graph for {@code configuration}
     * via {@link ResolutionResult} rather than
     * {@code ResolvedConfiguration.getResolvedArtifacts()} — the latter
     * forces artifact (jar file) resolution, which is unnecessary here and
     * can fail for reasons that have nothing to do with the *metadata*
     * this model reports (missing classifiers, packaging quirks).
     * {@code getAllComponents()} already flattens the whole graph
     * (direct + transitive, deduplicated) into one set, matching what
     * seção 10 asks for: "dependências resolvidas por configuração",
     * without this bridge needing to walk the graph recursively itself.
     */
    private void collectResolvedDependencies(
            Configuration configuration, String configurationName, List<JvmfastDependency> out) {
        ResolutionResult resolutionResult = configuration.getIncoming().getResolutionResult();
        ResolvedComponentResult root = resolutionResult.getRoot();
        for (ResolvedComponentResult component : resolutionResult.getAllComponents()) {
            if (component.getId().equals(root.getId())) {
                continue;
            }
            ModuleVersionIdentifier moduleVersion = component.getModuleVersion();
            if (moduleVersion == null) {
                // Project dependencies (other subprojects) and other non-module
                // components have no group:artifact:version identity to report —
                // multi-project graphs are Fase 5 scope (docs/architecture.md
                // seção 10), so these are skipped rather than guessed at.
                continue;
            }
            String coordinate = moduleVersion.getGroup() + ":" + moduleVersion.getModule().getName();
            out.add(new DefaultJvmfastDependency(coordinate, moduleVersion.getVersion(), configurationName));
        }
    }
}
