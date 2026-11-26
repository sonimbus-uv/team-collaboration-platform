import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class SharedFilesScreen extends StatefulWidget {
  const SharedFilesScreen({super.key});

  @override
  State<SharedFilesScreen> createState() => _SharedFilesScreenState();
}

class _SharedFilesScreenState extends State<SharedFilesScreen> {
  final TextEditingController _searchController = TextEditingController();

  String _selectedType = 'Todos';
  String _selectedChannel = 'Todos los canales';
  String _selectedDate = 'Cualquier fecha';

  // Cambia este valor a 'false' para ver los datos de prueba, o 'true' para el estado vacío.
  final bool _isEmpty = false;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
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
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        iconTheme: IconThemeData(color: colorScheme.onSurface),
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(1),
          child: Divider(height: 1, color: appColors.divider),
        ),
        title: Text(
          'Archivos compartidos',
          style: TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.bold,
            color: colorScheme.onSurface,
          ),
        ),
      ),
      body: Column(
        children: [
          _buildFiltersSection(colorScheme, appColors),
          Divider(height: 1, color: appColors.divider),
          Expanded(
            child: _isEmpty
                ? _buildEmptyState(colorScheme)
                : _buildFilesList(colorScheme, appColors),
          ),
        ],
      ),
    );
  }

  Widget _buildFiltersSection(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      padding: const EdgeInsets.all(16.0),
      color: appColors.highlight,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: TextField(
                  controller: _searchController,
                  cursorColor: colorScheme.primary,
                  style: TextStyle(color: colorScheme.onSurface),
                  decoration: InputDecoration(
                    hintText: 'Nombre del archivo',
                    prefixIcon: Icon(
                      Icons.search,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 12,
                    ),
                    filled: true,
                    fillColor: colorScheme.surface,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              ElevatedButton(
                onPressed: () {
                  // TODO: Implementar búsqueda real de archivos compartidos (MinIO / Core Service)
                },
                style: ElevatedButton.styleFrom(
                  backgroundColor: colorScheme.primary,
                  foregroundColor: colorScheme.onPrimary,
                  elevation: 0,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 24,
                    vertical: 14,
                  ),
                ),
                child: const Text(
                  'Buscar',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            children: [
              Expanded(
                child: _buildDropdown(
                  'Tipo',
                  [
                    'Todos',
                    'Documentos',
                    'Imágenes',
                    'Hojas de cálculo',
                    'Otros',
                  ],
                  _selectedType,
                  (v) => setState(() => _selectedType = v!),
                  colorScheme,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _buildDropdown(
                  'Canal',
                  ['Todos los canales', '# general', '# tareas'],
                  _selectedChannel,
                  (v) => setState(() => _selectedChannel = v!),
                  colorScheme,
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          _buildDropdown(
            'Fecha',
            [
              'Cualquier fecha',
              'Últimos 7 días',
              'Últimos 30 días',
              'Rango personalizado…',
            ],
            _selectedDate,
            (v) => setState(() => _selectedDate = v!),
            colorScheme,
          ),
        ],
      ),
    );
  }

  Widget _buildDropdown(
    String label,
    List<String> options,
    String currentValue,
    ValueChanged<String?> onChanged,
    ColorScheme colorScheme,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            fontSize: 13,
            color: colorScheme.onSurface,
          ),
        ),
        const SizedBox(height: 6),
        Container(
          height: 48,
          padding: const EdgeInsets.symmetric(horizontal: 12),
          decoration: BoxDecoration(
            border: Border.all(color: colorScheme.outline),
            borderRadius: BorderRadius.circular(8),
            color: colorScheme.surface,
          ),
          child: DropdownButtonHideUnderline(
            child: DropdownButton<String>(
              value: currentValue,
              isExpanded: true,
              dropdownColor: colorScheme.surface,
              icon: Icon(
                Icons.arrow_drop_down,
                color: colorScheme.onSurfaceVariant,
              ),
              style: TextStyle(fontSize: 14, color: colorScheme.onSurface),
              items: options
                  .map(
                    (s) => DropdownMenuItem(
                      value: s,
                      child: Text(s, style: const TextStyle(fontSize: 14)),
                    ),
                  )
                  .toList(),
              onChanged: onChanged,
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildEmptyState(ColorScheme colorScheme) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 32.0),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            Icons.folder_off_outlined,
            size: 64,
            color: colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 24),
          Text(
            'No hay archivos que coincidan',
            style: TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          Text(
            'Prueba con otros filtros o comparte un archivo desde un canal con el botón Adjuntar.',
            style: TextStyle(fontSize: 14, color: colorScheme.onSurfaceVariant),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          ElevatedButton(
            onPressed: () {
              setState(() {
                _searchController.clear();
                _selectedType = 'Todos';
                _selectedChannel = 'Todos los canales';
                _selectedDate = 'Cualquier fecha';
              });
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: colorScheme.primary,
              foregroundColor: colorScheme.onPrimary,
              elevation: 0,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
              ),
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
            ),
            child: const Text(
              'Limpiar filtros',
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildFilesList(ColorScheme colorScheme, AppThemeColors appColors) {
    return ListView.separated(
      padding: const EdgeInsets.all(16),
      itemCount: 4,
      separatorBuilder: (context, index) => const SizedBox(height: 12),
      itemBuilder: (context, index) {
        return Card(
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(color: appColors.divider),
          ),
          color: colorScheme.surface,
          child: InkWell(
            borderRadius: BorderRadius.circular(12),
            hoverColor: appColors.highlight,
            highlightColor: appColors.highlight,
            onTap: () =>
                _showFilePreviewBottomSheet(context, colorScheme, appColors),
            child: Padding(
              padding: const EdgeInsets.all(16.0),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Container(
                    width: 48,
                    height: 48,
                    decoration: BoxDecoration(
                      color: appColors.selectedItem,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Icon(
                      Icons.picture_as_pdf,
                      color: colorScheme.primary,
                    ),
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'plantilla_especificacion.pdf',
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            fontSize: 15,
                            color: colorScheme.onSurface,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'Compartido por Ana López en # general',
                          style: TextStyle(
                            fontSize: 13,
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          '20/09/2026 • 1.2 MB',
                          style: TextStyle(
                            fontSize: 12,
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  Icon(Icons.more_vert, color: colorScheme.onSurfaceVariant),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  void _showFilePreviewBottomSheet(
    BuildContext context,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      backgroundColor: colorScheme.surface,
      builder: (sheetContext) {
        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(24.0),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text(
                      'Vista previa',
                      style: TextStyle(
                        fontSize: 18,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.onSurface,
                      ),
                    ),
                    IconButton(
                      tooltip: 'Cerrar vista previa',
                      icon: Icon(
                        Icons.close,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      onPressed: () => Navigator.pop(sheetContext),
                      padding: EdgeInsets.zero,
                      constraints: const BoxConstraints(),
                    ),
                  ],
                ),
                const SizedBox(height: 24),
                Container(
                  height: 200,
                  decoration: BoxDecoration(
                    color: appColors.highlight,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: appColors.divider),
                  ),
                  alignment: Alignment.center,
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Icon(
                        Icons.picture_as_pdf,
                        size: 64,
                        color: colorScheme.primary,
                      ),
                      const SizedBox(height: 12),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 4,
                        ),
                        decoration: BoxDecoration(
                          color: appColors.selectedItem,
                          borderRadius: BorderRadius.circular(16),
                        ),
                        child: Text(
                          'Documento',
                          style: TextStyle(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.bold,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),
                Text(
                  'plantilla_especificacion.pdf',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.bold,
                    color: colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  'Compartido por Ana López en # general el 20 de Septiembre, 2026. Tamaño: 1.2 MB.',
                  style: TextStyle(
                    fontSize: 14,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 24),
                ElevatedButton.icon(
                  onPressed: () {
                    // TODO: Descargar archivo desde MinIO / Core Service
                  },
                  icon: const Icon(Icons.download),
                  label: const Text(
                    'Descargar',
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: colorScheme.primary,
                    foregroundColor: colorScheme.onPrimary,
                    elevation: 0,
                    padding: const EdgeInsets.symmetric(vertical: 16),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: () {
                    // TODO: Navegar al canal / mensaje origen
                    Navigator.pop(sheetContext);
                  },
                  icon: const Icon(Icons.chat_bubble_outline),
                  label: const Text('Ir al mensaje'),
                  style: OutlinedButton.styleFrom(
                    foregroundColor: colorScheme.onSurface,
                    side: BorderSide(color: colorScheme.outline),
                    padding: const EdgeInsets.symmetric(vertical: 16),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}
