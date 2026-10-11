package mx.uv.sonimbus.desktop.ui.reports;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.ComboBox;
import javafx.scene.control.DatePicker;
import javafx.scene.control.Label;
import javafx.scene.control.MenuButton;
import javafx.scene.control.MenuItem;
import javafx.scene.control.TableView;
import javafx.scene.layout.HBox;
import javafx.scene.layout.StackPane;

public class ReportsController {

    // Parámetros
    @FXML private ComboBox<String> reportTypeCombo;
    @FXML private DatePicker startDatePicker;
    @FXML private DatePicker endDatePicker;
    @FXML private Button queryButton;
    @FXML private MenuButton exportMenuButton;
    @FXML private MenuItem exportPdfMenuItem;
    @FXML private MenuItem exportCsvMenuItem;

    // Resultado
    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private Label reportTitleLabel;
    @FXML private Label reportRangeLabel;
    @FXML private StackPane chartPane;
    @FXML private Label chartPlaceholderLabel;
    @FXML private TableView<Object> reportTable;         // TODO: modelo de fila del reporte

    @FXML
    private void initialize() {
        // TODO CU-29: las columnas de reportTable dependen del tipo de reporte: crearlas en
        //             código al consultar (TableColumn<Object, Object>), con una fila "Total"
        // TODO CU-29: proponer un periodo inicial (p. ej. el mes en curso) en
        //             startDatePicker/endDatePicker
    }

    @FXML
    private void onQuery() {
        // TODO CU-29: validar tipo y periodo (inicio <= fin; errorBox); consultar el reporte;
        //             llenar reportTitleLabel, reportRangeLabel y reportTable; crear la gráfica
        //             (BarChart/PieChart, con accessibleText que la describa), colocarla en
        //             chartPane en lugar de chartPlaceholderLabel y habilitar exportMenuButton
    }

    @FXML
    private void onExportPdf() {
        // TODO CU-30: exportar a PDF el reporte consultado (FileChooser para el destino)
    }

    @FXML
    private void onExportCsv() {
        // TODO CU-30: exportar a CSV el reporte consultado (FileChooser para el destino)
    }
}
