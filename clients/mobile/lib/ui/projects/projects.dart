import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class ProjectsAndTasksScreen extends StatefulWidget {
  const ProjectsAndTasksScreen({super.key});

  @override
  State<ProjectsAndTasksScreen> createState() => _ProjectsAndTasksScreenState();
}

class _ProjectsAndTasksScreenState extends State<ProjectsAndTasksScreen> {
  String _selectedAssignee = 'Todos';
  final List<String> _assignees = [
    'Todos',
    'Luis Vázquez',
    'Ana López',
    'Carlos Ruiz',
  ];

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
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Proyectos y tareas',
              style: TextStyle(
                fontSize: 18,
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
            ),
            Text(
              'Selecciona un proyecto',
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.normal,
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: Icon(Icons.filter_list, color: colorScheme.onSurface),
            tooltip: 'Filtrar responsable',
            onPressed: () => _showFilterDialog(colorScheme, appColors),
          ),
          PopupMenuButton<String>(
            icon: Icon(Icons.add, color: colorScheme.primary),
            tooltip: 'Crear nuevo',
            color: colorScheme.surface,
            surfaceTintColor: Colors.transparent,
            offset: const Offset(0, 45),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
              side: BorderSide(color: appColors.divider),
            ),
            onSelected: (String value) {
              if (value == 'project') {
                // TODO: Implementar lógica de creación de proyecto acorde al MVP
              } else if (value == 'task') {
                // TODO: Implementar lógica de creación de tarea acorde al MVP
              }
            },
            itemBuilder: (BuildContext menuContext) => <PopupMenuEntry<String>>[
              PopupMenuItem<String>(
                value: 'project',
                child: ListTile(
                  leading: Icon(
                    Icons.folder_open_outlined,
                    size: 22,
                    color: colorScheme.primary,
                  ),
                  title: Text(
                    'Nuevo proyecto',
                    style: TextStyle(
                      fontSize: 15,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  contentPadding: EdgeInsets.zero,
                  visualDensity: VisualDensity.compact,
                ),
              ),
              PopupMenuItem<String>(
                value: 'task',
                child: ListTile(
                  leading: Icon(
                    Icons.check_circle_outline,
                    size: 22,
                    color: colorScheme.primary,
                  ),
                  title: Text(
                    'Nueva tarea',
                    style: TextStyle(
                      fontSize: 15,
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
      drawer: _buildProjectsDrawer(colorScheme, appColors),
      body: _buildKanbanBoard(colorScheme, appColors),
    );
  }

  Widget _buildProjectsDrawer(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Drawer(
      backgroundColor: appColors.sidePanel,
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 24, 16, 12),
              child: Text(
                'PROYECTOS (0)',
                style: TextStyle(
                  fontWeight: FontWeight.bold,
                  color: colorScheme.onSurfaceVariant,
                  fontSize: 13,
                ),
              ),
            ),
            Divider(height: 1, color: appColors.divider),
            Expanded(
              child: ListView(
                padding: EdgeInsets.zero,
                children: [
                  ListTile(
                    title: Text(
                      'No hay proyectos',
                      style: TextStyle(color: colorScheme.onSurfaceVariant),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildKanbanBoard(ColorScheme colorScheme, AppThemeColors appColors) {
    return ListView(
      scrollDirection: Axis.vertical,
      padding: const EdgeInsets.all(16.0),
      children: [
        _buildKanbanColumn(
          title: 'Pendiente',
          count: 0,
          icon: Icons.circle_outlined,
          iconColor: colorScheme.onSurfaceVariant,
          backgroundColor: appColors.highlight,
          chipBackgroundColor: colorScheme.surface,
          chipTextColor: colorScheme.onSurface,
          chipBorderColor: appColors.divider,
          colorScheme: colorScheme,
          appColors: appColors,
        ),
        const SizedBox(height: 16),
        _buildKanbanColumn(
          title: 'En progreso',
          count: 0,
          icon: Icons.access_time,
          iconColor: colorScheme.primary,
          backgroundColor: appColors.selectedItem.withValues(alpha: 0.45),
          chipBackgroundColor: appColors.selectedItem,
          chipTextColor: colorScheme.primary,
          chipBorderColor: colorScheme.primary.withValues(alpha: 0.4),
          colorScheme: colorScheme,
          appColors: appColors,
        ),
        const SizedBox(height: 16),
        _buildKanbanColumn(
          title: 'Completada',
          count: 0,
          icon: Icons.check_circle_outline,
          iconColor: appColors.mintForeground,
          backgroundColor: appColors.mintBackground.withValues(alpha: 0.45),
          chipBackgroundColor: appColors.mintBackground,
          chipTextColor: appColors.mintForeground,
          chipBorderColor: appColors.mintForeground.withValues(alpha: 0.35),
          colorScheme: colorScheme,
          appColors: appColors,
        ),
      ],
    );
  }

  Widget _buildKanbanColumn({
    required String title,
    required int count,
    required IconData icon,
    required Color iconColor,
    required Color backgroundColor,
    required Color chipBackgroundColor,
    required Color chipTextColor,
    required Color chipBorderColor,
    required ColorScheme colorScheme,
    required AppThemeColors appColors,
  }) {
    return Container(
      width: double.infinity,
      height: 190,
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: appColors.divider),
      ),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(14.0),
            child: Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: chipBackgroundColor,
                    borderRadius: BorderRadius.circular(20),
                    border: Border.all(color: chipBorderColor),
                  ),
                  child: Row(
                    children: [
                      Icon(icon, color: iconColor, size: 16),
                      const SizedBox(width: 6),
                      Text(
                        title,
                        style: TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 13,
                          color: chipTextColor,
                        ),
                      ),
                    ],
                  ),
                ),
                const Spacer(),
                Semantics(
                  label: '$count tareas en $title',
                  child: Text(
                    '$count',
                    style: TextStyle(
                      fontWeight: FontWeight.bold,
                      color: colorScheme.onSurfaceVariant,
                      fontSize: 15,
                    ),
                  ),
                ),
              ],
            ),
          ),
          Expanded(
            child: ListView(
              padding: const EdgeInsets.symmetric(horizontal: 10),
              // Espacio reservado para las tarjetas de tareas de la columna
            ),
          ),
          const SizedBox(height: 12),
        ],
      ),
    );
  }

  void _showFilterDialog(ColorScheme colorScheme, AppThemeColors appColors) {
    showDialog(
      context: context,
      builder: (dialogContext) {
        return AlertDialog(
          backgroundColor: colorScheme.surface,
          surfaceTintColor: Colors.transparent,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(color: appColors.divider),
          ),
          title: Text(
            'Responsable',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
          ),
          content: DropdownButtonFormField<String>(
            initialValue: _selectedAssignee,
            dropdownColor: colorScheme.surface,
            iconEnabledColor: colorScheme.onSurfaceVariant,
            style: TextStyle(color: colorScheme.onSurface, fontSize: 14),
            decoration: const InputDecoration(
              contentPadding: EdgeInsets.symmetric(
                horizontal: 12,
                vertical: 10,
              ),
            ),
            items: _assignees.map((String assignee) {
              return DropdownMenuItem<String>(
                value: assignee,
                child: Text(assignee),
              );
            }).toList(),
            onChanged: (String? newValue) {
              if (newValue != null) {
                setState(() {
                  _selectedAssignee = newValue;
                });
                Navigator.pop(dialogContext);
              }
            },
          ),
        );
      },
    );
  }
}
