plugins {
    `java-library`
}

dependencies {
    api(project(":core"))
    api(project(":signaling"))

    api("dev.onvoid.webrtc:webrtc-java:0.18.0")

    // Protobuf runtime
    implementation("com.google.protobuf:protobuf-java:3.25.5")

    // Testing
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.2")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
    testRuntimeOnly("dev.onvoid.webrtc:webrtc-java:0.18.0:linux-x86_64")
}
