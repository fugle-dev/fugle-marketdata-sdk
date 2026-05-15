plugins {
    `java-library`
    `maven-publish`
}

group = "tw.com.fugle"
version = providers.gradleProperty("projectVersion").getOrElse("3.0.0-dev")

publishing {
    publications {
        create<MavenPublication>("gpr") {
            from(components["java"])
            groupId = "tw.com.fugle"
            artifactId = "marketdata-java"

            pom {
                name.set("Fugle Market Data SDK for Java")
                description.set("UniFFI-generated Java bindings for Fugle Market Data SDK")
                url.set("https://github.com/kevinypfan/fugle-marketdata-sdk")

                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }

                developers {
                    developer {
                        id.set("kevinypfan")
                        name.set("Fugle Development Team")
                    }
                }

                scm {
                    connection.set("scm:git:git://github.com/kevinypfan/fugle-marketdata-sdk.git")
                    developerConnection.set("scm:git:ssh://github.com:kevinypfan/fugle-marketdata-sdk.git")
                    url.set("https://github.com/kevinypfan/fugle-marketdata-sdk")
                }
            }
        }
    }
    repositories {
        maven {
            name = "GitHubPackages"
            url = uri("https://maven.pkg.github.com/kevinypfan/fugle-marketdata-sdk")
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

tasks.test {
    useJUnitPlatform()
}

// Source sets - generated code lives in generated/ package
sourceSets {
    main {
        java {
            srcDirs("src/main/java", "examples")
        }
    }
}
