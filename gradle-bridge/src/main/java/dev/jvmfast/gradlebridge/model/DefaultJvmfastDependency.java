package dev.jvmfast.gradlebridge.model;

import java.io.Serializable;
import java.util.Objects;

/** Plain, {@link Serializable} implementation of {@link JvmfastDependency}. */
public final class DefaultJvmfastDependency implements JvmfastDependency, Serializable {

    private final String coordinate;
    private final String version;
    private final String configuration;

    public DefaultJvmfastDependency(String coordinate, String version, String configuration) {
        this.coordinate = Objects.requireNonNull(coordinate, "coordinate");
        this.version = Objects.requireNonNull(version, "version");
        this.configuration = Objects.requireNonNull(configuration, "configuration");
    }

    @Override
    public String getCoordinate() {
        return coordinate;
    }

    @Override
    public String getVersion() {
        return version;
    }

    @Override
    public String getConfiguration() {
        return configuration;
    }
}
