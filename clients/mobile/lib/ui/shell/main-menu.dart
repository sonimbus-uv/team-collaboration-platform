import 'package:flutter/material.dart';
import 'package:mobile/ui/files/shared-files.dart';
import 'package:mobile/ui/groups/create-group.dart';
import 'package:mobile/ui/groups/join-group.dart';
import 'package:mobile/ui/projects/projects.dart';
import 'package:mobile/ui/reports/reports.dart';
import 'package:mobile/ui/theme/app-theme-extension.dart';

class MainShellScreen extends StatefulWidget {
  const MainShellScreen({super.key});

  @override
  State<MainShellScreen> createState() => _MainShellScreenState();
}

class _MainShellScreenState extends State<MainShellScreen> {
  int _currentIndex = 0;

  // Vistas controladas por el menú lateral (preservan su estado en IndexedStack)
  late final List<Widget> _views = [
    Builder(
      builder: (context) => Center(
        child: Text(
          'Selecciona un canal para empezar',
          style: TextStyle(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
            fontSize: 15,
          ),
        ),
      ),
    ),
    const SharedFilesScreen(),
    const ProjectsAndTasksScreen(),
    const ReportsScreen(),
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
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(1),
          child: Divider(height: 1, color: appColors.divider),
        ),
        iconTheme: IconThemeData(color: colorScheme.onSurface),
        actions: [
          IconButton(
            tooltip: 'Notificaciones',
            icon: Icon(
              Icons.notifications_none,
              size: 26,
              color: colorScheme.onSurface,
            ),
            onPressed: () => Navigator.pushNamed(context, '/notifications'),
          ),
          Padding(
            padding: const EdgeInsets.only(right: 16.0, left: 8.0),
            child: Semantics(
              button: true,
              label: 'Mi perfil',
              child: GestureDetector(
                onTap: () => Navigator.pushNamed(context, '/profile'),
                child: Container(
                  width: 32,
                  height: 32,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    border: Border.all(color: colorScheme.primary, width: 1.5),
                    color: appColors.selectedItem,
                  ),
                  alignment: Alignment.center,
                  child: Text(
                    '?',
                    style: TextStyle(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.bold,
                      fontSize: 16,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
      drawer: _buildDrawer(colorScheme, appColors),
      body: IndexedStack(index: _currentIndex, children: _views),
    );
  }

  Widget _buildDrawer(ColorScheme colorScheme, AppThemeColors appColors) {
    return Drawer(
      backgroundColor: appColors.sidePanel,
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 20, 16, 20),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'Selecciona un grupo',
                    style: TextStyle(
                      color: colorScheme.onSurface,
                      fontSize: 15,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  IconButton(
                    tooltip: 'Configuración del grupo',
                    onPressed: () {
                      Navigator.pushNamed(context, '/group-settings');
                    },
                    style: IconButton.styleFrom(
                      side: BorderSide(color: colorScheme.outline),
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(6),
                      ),
                      padding: const EdgeInsets.all(6),
                      minimumSize: const Size(32, 32),
                    ),
                    icon: Icon(
                      Icons.settings_outlined,
                      color: colorScheme.onSurface,
                      size: 20,
                    ),
                  ),
                ],
              ),
            ),
            Divider(color: appColors.divider, height: 1),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(vertical: 16),
                children: [
                  _buildSectionHeader('CANALES DE TEXTO', colorScheme),
                  const SizedBox(height: 32),
                  _buildSectionHeader('CANALES DE VOZ', colorScheme),
                  const SizedBox(height: 48),
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 20,
                      vertical: 8,
                    ),
                    child: Text(
                      'GRUPO',
                      style: TextStyle(
                        color: colorScheme.onSurfaceVariant,
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                        letterSpacing: 0.5,
                      ),
                    ),
                  ),
                  _buildSidebarItem(
                    'Archivos compartidos',
                    1,
                    Icons.folder_outlined,
                    colorScheme,
                    appColors,
                  ),
                  _buildSidebarItem(
                    'Proyectos y tareas',
                    2,
                    Icons.checklist_outlined,
                    colorScheme,
                    appColors,
                  ),
                  _buildSidebarItem(
                    'Reportes',
                    3,
                    Icons.bar_chart_outlined,
                    colorScheme,
                    appColors,
                  ),
                ],
              ),
            ),
            Divider(color: appColors.divider, height: 1),
            Padding(
              padding: const EdgeInsets.all(20.0),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Container(
                  width: 46,
                  height: 46,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: colorScheme.surface,
                    border: Border.all(color: colorScheme.outline, width: 1.5),
                  ),
                  child: IconButton(
                    tooltip: 'Crear o unirse a un grupo',
                    icon: Icon(Icons.add, color: colorScheme.primary, size: 22),
                    onPressed: () {
                      _showGroupOptionsModal(context, colorScheme, appColors);
                    },
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  void _showGroupOptionsModal(
    BuildContext context,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    showModalBottomSheet(
      context: context,
      backgroundColor: colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (BuildContext sheetContext) {
        return SafeArea(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(height: 8),
              Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: appColors.divider,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(height: 16),
              ListTile(
                leading: Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: appColors.selectedItem,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(
                    Icons.group_add_outlined,
                    color: colorScheme.primary,
                  ),
                ),
                title: Text(
                  'Crear grupo',
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurface,
                  ),
                ),
                subtitle: Text(
                  'Inicia una nueva comunidad',
                  style: TextStyle(color: colorScheme.onSurfaceVariant),
                ),
                onTap: () {
                  Navigator.pop(sheetContext);
                  showDialog(
                    context: context,
                    builder: (BuildContext dialogContext) =>
                        const CreateGroupDialog(),
                  );
                },
              ),
              ListTile(
                leading: Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: appColors.highlight,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(
                    Icons.login_outlined,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                title: Text(
                  'Unirse a un grupo',
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    color: colorScheme.onSurface,
                  ),
                ),
                subtitle: Text(
                  'Ingresa con un código de invitación',
                  style: TextStyle(color: colorScheme.onSurfaceVariant),
                ),
                onTap: () {
                  Navigator.pop(sheetContext);
                  showDialog(
                    context: context,
                    builder: (BuildContext dialogContext) =>
                        const JoinGroupDialog(),
                  );
                },
              ),
              const SizedBox(height: 16),
            ],
          ),
        );
      },
    );
  }

  Widget _buildSectionHeader(String title, ColorScheme colorScheme) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            title,
            style: TextStyle(
              color: colorScheme.onSurfaceVariant,
              fontSize: 12,
              fontWeight: FontWeight.bold,
              letterSpacing: 0.5,
            ),
          ),
          Icon(Icons.add, color: colorScheme.onSurfaceVariant, size: 16),
        ],
      ),
    );
  }

  Widget _buildSidebarItem(
    String title,
    int index,
    IconData icon,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    final isSelected = _currentIndex == index;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        hoverColor: appColors.highlight,
        highlightColor: appColors.highlight,
        onTap: () {
          setState(() => _currentIndex = index);
          Navigator.pop(context);
        },
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          decoration: BoxDecoration(
            color: isSelected ? appColors.selectedItem : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
          ),
          child: Row(
            children: [
              // Barra lateral indicadora del elemento activo (accesibilidad WCAG 2.2)
              Container(
                width: 4,
                height: 20,
                decoration: BoxDecoration(
                  color: isSelected ? colorScheme.primary : Colors.transparent,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(width: 10),
              Icon(
                icon,
                size: 18,
                color: isSelected
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  title,
                  style: TextStyle(
                    color: isSelected
                        ? colorScheme.onSurface
                        : colorScheme.onSurfaceVariant,
                    fontSize: 14,
                    fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
