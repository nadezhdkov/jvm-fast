package dev.jvmfast.gradlebridge;

import dev.jvmfast.gradlebridge.model.JvmfastDependency;
import dev.jvmfast.gradlebridge.model.JvmfastDependencyModel;
import dev.jvmfast.gradlebridge.model.JvmfastModule;
import org.gradle.tooling.BuildException;
import org.gradle.tooling.GradleConnectionException;
import org.gradle.tooling.GradleConnector;
import org.gradle.tooling.ProjectConnection;

import java.io.File;
import java.util.List;

/**
 * The Tooling API client-side driver (docs/architecture.md seção 10, steps
 * 2-4) — invoked by `jvmfast import-gradle` (Rust side, `src/gradleimport/`)
 * as {@code java -jar jvmfast-gradle-bridge.jar <project-dir> <init-script-path>}.
 * Opens a {@link ProjectConnection} to the target project, requests
 * {@link JvmfastDependencyModel} (built server-side by
 * {@link JvmfastModelBuilder} through the plugin the init-script applies),
 * and prints <b>only</b> the resulting JSON to this process's own stdout —
 * never {@code gradlew}'s, per seção 10's explicit rejection of
 * stdout-text-parsing as a migration mechanism. The target build's own
 * console output is redirected to this process's stderr instead
 * (discardable, never mixed into the JSON channel).
 */
public final class Main {

    private Main() {
    }

    public static void main(String[] args) {
        if (args.length != 2) {
            System.err.println("usage: jvmfast-gradle-bridge.jar <project-dir> <init-script-path>");
            System.exit(2);
            return;
        }

        File projectDir = new File(args[0]);
        String initScriptPath = args[1];

        GradleConnector connector = GradleConnector.newConnector().forProjectDirectory(projectDir);
        try (ProjectConnection connection = connector.connect()) {
            JvmfastDependencyModel model = connection.model(JvmfastDependencyModel.class)
                    .withArguments("--init-script", initScriptPath)
                    .setStandardOutput(System.err)
                    .setStandardError(System.err)
                    .get();
            System.out.print(toJson(model));
            System.out.flush();
        } catch (BuildException e) {
            System.err.println("gradle build failed: " + e.getMessage());
            System.exit(3);
        } catch (GradleConnectionException e) {
            System.err.println("could not connect to the target Gradle build: " + e.getMessage());
            System.exit(4);
        } catch (RuntimeException e) {
            System.err.println("jvmfast-gradle-bridge failed: " + e.getMessage());
            System.exit(1);
        }
    }

    /**
     * Hand-rolled rather than pulling in Gson/Jackson — the model is a
     * small, fixed shape of strings and lists, and this stays free of an
     * extra dependency to shade into the client jar (see
     * `build.gradle.kts`'s shadowJar setup).
     */
    static String toJson(JvmfastDependencyModel model) {
        StringBuilder json = new StringBuilder();
        json.append("{\"modules\":[");
        List<JvmfastModule> modules = model.getModules();
        for (int i = 0; i < modules.size(); i++) {
            if (i > 0) {
                json.append(',');
            }
            appendModule(json, modules.get(i));
        }
        json.append("]}");
        return json.toString();
    }

    private static void appendModule(StringBuilder json, JvmfastModule module) {
        json.append("{\"name\":").append(quote(module.getName()))
                .append(",\"version\":").append(quote(module.getVersion()))
                .append(",\"dependencies\":[");
        List<JvmfastDependency> dependencies = module.getDependencies();
        for (int i = 0; i < dependencies.size(); i++) {
            if (i > 0) {
                json.append(',');
            }
            appendDependency(json, dependencies.get(i));
        }
        json.append("]}");
    }

    private static void appendDependency(StringBuilder json, JvmfastDependency dependency) {
        json.append("{\"coordinate\":").append(quote(dependency.getCoordinate()))
                .append(",\"version\":").append(quote(dependency.getVersion()))
                .append(",\"configuration\":").append(quote(dependency.getConfiguration()))
                .append('}');
    }

    private static String quote(String raw) {
        StringBuilder out = new StringBuilder(raw.length() + 2);
        out.append('"');
        for (int i = 0; i < raw.length(); i++) {
            char c = raw.charAt(i);
            switch (c) {
                case '"':
                    out.append("\\\"");
                    break;
                case '\\':
                    out.append("\\\\");
                    break;
                case '\n':
                    out.append("\\n");
                    break;
                case '\r':
                    out.append("\\r");
                    break;
                case '\t':
                    out.append("\\t");
                    break;
                default:
                    if (c < 0x20) {
                        out.append(String.format("\\u%04x", (int) c));
                    } else {
                        out.append(c);
                    }
            }
        }
        out.append('"');
        return out.toString();
    }
}
