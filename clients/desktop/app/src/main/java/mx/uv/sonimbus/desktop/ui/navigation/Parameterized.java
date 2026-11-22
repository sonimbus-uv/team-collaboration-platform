package mx.uv.sonimbus.desktop.ui.navigation;

public interface Parameterized<P> {

    Class<P> parametersType();

    void setParameters(P parameters);
}
