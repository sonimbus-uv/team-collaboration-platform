package mx.uv.sonimbus.desktop.ui.navigation;

import javafx.scene.Parent;

public interface ContentHost {

    void setContent(View view, Parent content);

    void clearContent();
}
