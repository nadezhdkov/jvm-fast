package dev.jvmfast.gradlebridge.model;

import java.io.Serializable;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/** Plain, {@link Serializable} implementation of {@link JvmfastDependencyModel}. */
public final class DefaultJvmfastDependencyModel implements JvmfastDependencyModel, Serializable {

    private final List<JvmfastModule> modules;

    public DefaultJvmfastDependencyModel(List<JvmfastModule> modules) {
        this.modules = Collections.unmodifiableList(Objects.requireNonNull(modules, "modules"));
    }

    @Override
    public List<JvmfastModule> getModules() {
        return modules;
    }
}
