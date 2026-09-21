plugins {
    `java-library`
    `maven-publish`
}

group = "tw.com.fugle"
version = providers.gradleProperty("projectVersion").getOrElse("0.2.0-rc.7")

publishing {
    publications {
        create<MavenPublication>("gpr") {
            from(components["java"])
            groupId = "tw.com.fugle"
            artifactId = "marketdata-java"

            pom {
                name.set("Fugle Market Data SDK for Java")
                description.set("UniFFI-generated Java bindings for Fugle Market Data SDK")
                url.set("https://github.com/fugle-dev/fugle-marketdata-sdk")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }

                developers {
                    developer {
                        id.set("fugle-dev")
                        name.set("Fugle Development Team")
                    }
                }

                scm {
                    connection.set("scm:git:git://github.com/fugle-dev/fugle-marketdata-sdk.git")
                    developerConnection.set("scm:git:ssh://github.com:fugle-dev/fugle-marketdata-sdk.git")
                    url.set("https://github.com/fugle-dev/fugle-marketdata-sdk")
                }
            }
        }
    }
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/fugle-dev/fugle-marketdata-sdk")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                password = System.getenv("GITHUB_TOKEN")
            }
        }
    }
}

java {
    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
}

repositories {
    mavenCentral()
}

dependencies {
    // JNA required for UniFFI-generated bindings
    implementation("net.java.dev.jna:jna:5.14.0")

    // Testing
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.0")
}

// `gradle test` has no tag flags on the command line, so tags are picked with
// properties: -PexcludeTags=integration / -PincludeTags=integration
// (comma-separated).
fun tagsProperty(name: String): List<String> =
    providers.gradleProperty(name).orNull
        ?.split(",")?.map { it.trim() }?.filter { it.isNotEmpty() }
        .orEmpty()

tasks.test {
    useJUnitPlatform {
        tagsProperty("includeTags").takeIf { it.isNotEmpty() }?.let { includeTags(*it.toTypedArray()) }
        tagsProperty("excludeTags").takeIf { it.isNotEmpty() }?.let { excludeTags(*it.toTypedArray()) }
    }

    // Tests that construct clients load the native library through JNA.
    // Defaults to the workspace's cargo release output; override with
    // -PnativeLibDir=<dir>. A missing directory only makes those tests skip.
    val nativeLibDir = providers.gradleProperty("nativeLibDir")
        .getOrElse(rootDir.resolve("../../target/release").canonicalPath)
    systemProperty("jna.library.path", nativeLibDir)

    // CI passes -PrequireNative so a library that fails to load fails those
    // tests instead of silently skipping them.
    if (providers.gradleProperty("requireNative").isPresent) {
        systemProperty("fugle.requireNative", "true")
    }
}

// Source sets - generated code lives in generated/ package
sourceSets {
    main {
        java {
            srcDirs("src/main/java", "examples")
        }
    }
}
