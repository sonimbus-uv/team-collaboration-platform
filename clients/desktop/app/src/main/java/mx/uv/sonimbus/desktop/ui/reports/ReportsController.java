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

    @FXML private ComboBox<String> reportTypeCombo;
    @FXML private DatePicker startDatePicker;
    @FXML private DatePicker endDatePicker;
    @FXML private Button queryButton;
    @FXML private MenuButton exportMenuButton;
    @FXML private MenuItem exportPdfMenuItem;
    @FXML private MenuItem exportCsvMenuItem;

    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private Label reportTitleLabel;
    @FXML private Label reportRangeLabel;
    @FXML private StackPane chartPane;
    @FXML private Label chartPlaceholderLabel;
    @FXML private TableView<Object> reportTable;

    @FXML
    private void initialize() {
    }

    @FXML
    private void onQuery() {
    }

    @FXML
    private void onExportPdf() {
    }

    @FXML
    private void onExportCsv() {
    }
}
