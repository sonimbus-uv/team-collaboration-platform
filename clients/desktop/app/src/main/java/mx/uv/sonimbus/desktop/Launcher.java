package mx.uv.sonimbus.desktop;

/** Punto de entrada que no extiende Application, para ejecutar desde el IDE o un JAR. */
public final class Launcher {

    private Launcher() {
    }

    public static void main(String[] args) {
        App.main(args);
    }
}
