# Security policy

jvm-fast is a dependency, JDK, and build manager for Java. Due to the design of the Java/Maven
ecosystem and the nature of source-level builds, there are cases where jvm-fast will run
arbitrary code by design. For example:

- jvm-fast invokes `javac`/`java` on the system to compile and run project code
- jvm-fast downloads and runs the JUnit Platform Console Standalone to execute tests
- jvm-fast resolves and downloads artifacts (including transitive dependencies) from configured
  Maven repositories, whose build/annotation-processing steps may execute code

These are not considered vulnerabilities in jvm-fast. If you think jvm-fast's stance in these
areas can be hardened, please file an issue for a new feature.

If you believe you have found a vulnerability that is in scope for the project — for example, in
lockfile parsing, checksum verification, cache handling, or the resolution/mediation logic —
please contact us as described in [SECURITY_CONTACT_URL].

<!--
  TODO: replace [SECURITY_CONTACT_URL] with the real disclosure channel once one exists
  (e.g., a private security-advisory form or an org-level SECURITY.md, following the pattern
  astral-sh/uv uses with astral-sh/.github).
-->
