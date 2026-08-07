plugins {
    java
}

group = "com.example"
version = "1.2.3"

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.slf4j:slf4j-api:2.0.16")
    testImplementation("junit:junit:4.13.2")
}
