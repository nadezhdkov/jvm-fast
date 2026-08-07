package dev.jvmfast.gradlebridge.model;

import java.io.Serializable;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/** Plain, {@link Serializable} implementation of {@link JvmfastModule}. */
public final class DefaultJvmfastModule implements JvmfastModule, Serializable {

    private final String name;
    private final String version;
    private final List<JvmfastDependency> dependencies;

    public DefaultJvmfastModule(String name, String version, List<JvmfastDependency> dependencies) {
        this.name = Objects.requireNonNull(name, "name");
        this.version = Objects.requireNonNull(version, "version");
        this.dependencies = Collections.unmodifiableList(dependencies);
    }

    @Override
    public String getName() {
        return name;
    }

    @Override
    public String getVersion() {
        return version;
    }

    @Override
    public List<JvmfastDependency> getDependencies() {
        return dependencies;
    }
}
