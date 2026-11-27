import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class ReportsScreen extends StatefulWidget {
  const ReportsScreen({super.key});

  @override
  State<ReportsScreen> createState() => _ReportsScreenState();
}

class _ReportsScreenState extends State<ReportsScreen> {
  String? _selectedReportType = 'Participación por miembro';
  final List<String> _reportTypes = [
    'Mensajes por periodo',
    'Participación por miembro',
    'Tareas por estado',
  ];

  DateTime _startDate = DateTime.now().subtract(const Duration(days: 14));
  DateTime _endDate = DateTime.now();
  bool _hasData = false;

  Future<void> _selectDate(BuildContext context, bool isStart) async {
    final DateTime? picked = await showDatePicker(
      context: context,
      initialDate: isStart ? _startDate : _endDate,
      firstDate: DateTime(2020),
      lastDate: DateTime(2030),
    );
    if (picked != null) {
      setState(() {
        if (isStart) {
          _startDate = picked;
        } else {
          _endDate = picked;
        }
      });
    }
  }

  String _formatDate(DateTime date) {
    return '${date.day.toString().padLeft(2, '0')}/${date.month.toString().padLeft(2, '0')}/${date.year}';
  }

  void _onQuery() {
    setState(() {
      _hasData = true;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final appColors = theme.extension<AppThemeColors>()!;

    return Scaffold(
      appBar: AppBar(
        backgroundColor: colorScheme.surface,
        foregroundColor: colorScheme.onSurface,
        elevation: 0,
        surfaceTintColor: Colors.transparent,
        iconTheme: IconThemeData(color: colorScheme.onSurface),
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(1),
          child: Divider(height: 1, color: appColors.divider),
        ),
        title: Text(
          'Reportes',
          style: TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.bold,
            color: colorScheme.onSurface,
          ),
        ),
        actions: [
          PopupMenuButton<String>(
            icon: Icon(
              Icons.download,
              color: _hasData
                  ? colorScheme.primary
                  : colorScheme.onSurfaceVariant,
            ),
            tooltip: 'Exportar reporte',
            enabled: _hasData,
            color: colorScheme.surface,
            surfaceTintColor: Colors.transparent,
            offset: const Offset(0, 45),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
              side: BorderSide(color: appColors.divider),
            ),
            onSelected: (String value) {
              // TODO: Lógica para exportar PDF o CSV (CU-30)
            },
            itemBuilder: (BuildContext menuContext) => <PopupMenuEntry<String>>[
              PopupMenuItem<String>(
                value: 'pdf',
                child: ListTile(
                  leading: Icon(
                    Icons.picture_as_pdf_outlined,
                    size: 20,
                    color: colorScheme.primary,
                  ),
                  title: Text(
                    'PDF (.pdf)',
                    style: TextStyle(
                      fontSize: 14,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  contentPadding: EdgeInsets.zero,
                  visualDensity: VisualDensity.compact,
                ),
              ),
              PopupMenuItem<String>(
                value: 'csv',
                child: ListTile(
                  leading: Icon(
                    Icons.table_chart_outlined,
                    size: 20,
                    color: colorScheme.primary,
                  ),
                  title: Text(
                    'CSV (.csv)',
                    style: TextStyle(
                      fontSize: 14,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  contentPadding: EdgeInsets.zero,
                  visualDensity: VisualDensity.compact,
                ),
              ),
            ],
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // ================= Parámetros del reporte (CU-29) =================
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: appColors.highlight,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: appColors.divider),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Tipo de reporte',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.bold,
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 6),
                  DropdownButtonFormField<String>(
                    initialValue: _selectedReportType,
                    dropdownColor: colorScheme.surface,
                    iconEnabledColor: colorScheme.onSurfaceVariant,
                    style: TextStyle(
                      fontSize: 14,
                      color: colorScheme.onSurface,
                    ),
                    decoration: InputDecoration(
                      filled: true,
                      fillColor: colorScheme.surface,
                      contentPadding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 10,
                      ),
                    ),
                    items: _reportTypes.map((String type) {
                      return DropdownMenuItem<String>(
                        value: type,
                        child: Text(type),
                      );
                    }).toList(),
                    onChanged: (String? newValue) {
                      setState(() {
                        _selectedReportType = newValue;
                      });
                    },
                  ),
                  const SizedBox(height: 14),
                  Row(
                    children: [
                      Expanded(
                        child: _buildDateSelector(
                          label: 'Desde',
                          dateText: _formatDate(_startDate),
                          onTap: () => _selectDate(context, true),
                          colorScheme: colorScheme,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: _buildDateSelector(
                          label: 'Hasta',
                          dateText: _formatDate(_endDate),
                          onTap: () => _selectDate(context, false),
                          colorScheme: colorScheme,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  SizedBox(
                    width: double.infinity,
                    child: ElevatedButton(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: colorScheme.primary,
                        foregroundColor: colorScheme.onPrimary,
                        elevation: 0,
                        padding: const EdgeInsets.symmetric(vertical: 12),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(8),
                        ),
                      ),
                      onPressed: _onQuery,
                      child: const Text(
                        'Consultar',
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          fontSize: 15,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 20),

            // ================= Resultados / Gráfica =================
            if (_hasData) ...[
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Text(
                      _selectedReportType ?? '',
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.onSurface,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    '${_formatDate(_startDate)} al ${_formatDate(_endDate)}',
                    style: TextStyle(
                      fontSize: 12,
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 12),
            ],

            // Contenedor de la gráfica
            Container(
              height: 220,
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: colorScheme.surface,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: appColors.divider),
              ),
              child: Center(
                child: _hasData
                    ? Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(
                            Icons.bar_chart,
                            size: 48,
                            color: colorScheme.primary,
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'Gráfica de $_selectedReportType',
                            style: TextStyle(
                              color: colorScheme.onSurface,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                        ],
                      )
                    : Text(
                        'Elige un tipo de reporte y un periodo, y pulsa Consultar',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: colorScheme.onSurfaceVariant,
                          fontSize: 14,
                        ),
                      ),
              ),
            ),
            const SizedBox(height: 20),

            // ================= Tabla de datos =================
            Container(
              decoration: BoxDecoration(
                color: colorScheme.surface,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: appColors.divider),
              ),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: _hasData
                    ? SingleChildScrollView(
                        scrollDirection: Axis.horizontal,
                        child: DataTable(
                          headingRowColor: WidgetStateProperty.all(
                            appColors.highlight,
                          ),
                          dividerThickness: 1,
                          headingTextStyle: TextStyle(
                            fontWeight: FontWeight.bold,
                            color: colorScheme.onSurface,
                          ),
                          dataTextStyle: TextStyle(
                            color: colorScheme.onSurface,
                          ),
                          columns: const [
                            DataColumn(label: Text('Miembro')),
                            DataColumn(label: Text('Asignadas')),
                            DataColumn(label: Text('Terminadas')),
                            DataColumn(label: Text('Vencidas')),
                          ],
                          rows: const [
                            DataRow(
                              cells: [
                                DataCell(Text('Luis Vázquez')),
                                DataCell(Text('6')),
                                DataCell(Text('5')),
                                DataCell(Text('0')),
                              ],
                            ),
                            DataRow(
                              cells: [
                                DataCell(Text('Ana López')),
                                DataCell(Text('6')),
                                DataCell(Text('4')),
                                DataCell(Text('1')),
                              ],
                            ),
                            DataRow(
                              cells: [
                                DataCell(Text('Carlos Ruiz')),
                                DataCell(Text('5')),
                                DataCell(Text('3')),
                                DataCell(Text('0')),
                              ],
                            ),
                          ],
                        ),
                      )
                    : Padding(
                        padding: const EdgeInsets.symmetric(vertical: 40),
                        child: Center(
                          child: Text(
                            'Sin datos para mostrar',
                            style: TextStyle(
                              color: colorScheme.onSurfaceVariant,
                              fontSize: 14,
                            ),
                          ),
                        ),
                      ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildDateSelector({
    required String label,
    required String dateText,
    required VoidCallback onTap,
    required ColorScheme colorScheme,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: TextStyle(
            fontSize: 12,
            fontWeight: FontWeight.bold,
            color: colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 6),
        Semantics(
          button: true,
          label: 'Seleccionar fecha $label: $dateText',
          child: InkWell(
            borderRadius: BorderRadius.circular(8),
            onTap: onTap,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
              decoration: BoxDecoration(
                color: colorScheme.surface,
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: colorScheme.outline),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    dateText,
                    style: TextStyle(
                      fontSize: 14,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  Icon(
                    Icons.calendar_today,
                    size: 16,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}
