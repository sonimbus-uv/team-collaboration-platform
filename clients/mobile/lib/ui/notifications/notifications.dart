import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class NotificationsScreen extends StatefulWidget {
  const NotificationsScreen({super.key});

  @override
  State<NotificationsScreen> createState() => _NotificationsScreenState();
}

class _NotificationsScreenState extends State<NotificationsScreen> {
  // Estado para el ToggleGroup (0 = Todas, 1 = No leídas)
  int _selectedFilter = 0;

  // Lista simulada para el historial de notificaciones
  final List<String> _notifications = [];

  void _handleMarkAllRead() {
    final appColors = Theme.of(context).extension<AppThemeColors>()!;

    // TODO: Implementar lógica (CU-31) para marcar todo como leído
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        backgroundColor: appColors.mintBackground,
        content: Row(
          children: [
            Icon(
              Icons.check_circle_outline,
              color: appColors.mintForeground,
              size: 20,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                'Todas las notificaciones marcadas como leídas',
                style: TextStyle(
                  color: appColors.mintForeground,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ],
        ),
      ),
    );
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
        bottom: PreferredSize(
          preferredSize: const Size.fromHeight(1),
          child: Divider(height: 1, color: appColors.divider),
        ),
        title: Text(
          'Notificaciones',
          style: TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.bold,
            color: colorScheme.onSurface,
          ),
        ),
        actions: [
          TextButton.icon(
            onPressed: _handleMarkAllRead,
            icon: Icon(Icons.done_all, color: colorScheme.primary, size: 20),
            label: Text(
              'Marcar leídas',
              style: TextStyle(
                color: colorScheme.primary,
                fontSize: 13,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
        ],
      ),
      body: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // ================= BARRA DE FILTROS =================
          Container(
            color: colorScheme.surface,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
            child: Row(
              children: [
                _buildSegmentButton(
                  'Todas',
                  0,
                  isLeft: true,
                  colorScheme: colorScheme,
                  appColors: appColors,
                ),
                _buildSegmentButton(
                  'No leídas (0)',
                  1,
                  isLeft: false,
                  colorScheme: colorScheme,
                  appColors: appColors,
                ),
              ],
            ),
          ),
          Divider(height: 1, color: appColors.divider),

          // ================= CONTENIDO CENTRAL =================
          Expanded(
            child: _notifications.isEmpty
                ? _buildEmptyState(colorScheme)
                : _buildNotificationsList(colorScheme, appColors),
          ),
        ],
      ),
    );
  }

  Widget _buildSegmentButton(
    String text,
    int index, {
    required bool isLeft,
    required ColorScheme colorScheme,
    required AppThemeColors appColors,
  }) {
    final bool isSelected = _selectedFilter == index;
    final borderRadius = isLeft
        ? const BorderRadius.only(
            topLeft: Radius.circular(6),
            bottomLeft: Radius.circular(6),
          )
        : const BorderRadius.only(
            topRight: Radius.circular(6),
            bottomRight: Radius.circular(6),
          );

    return Expanded(
      child: Semantics(
        button: true,
        selected: isSelected,
        label: text,
        child: InkWell(
          borderRadius: borderRadius,
          hoverColor: appColors.highlight,
          highlightColor: appColors.highlight,
          onTap: () => setState(() => _selectedFilter = index),
          child: Container(
            padding: const EdgeInsets.symmetric(vertical: 10),
            decoration: BoxDecoration(
              color: isSelected ? appColors.selectedItem : colorScheme.surface,
              border: Border.all(
                color: isSelected ? colorScheme.primary : colorScheme.outline,
                width: isSelected ? 1.5 : 1,
              ),
              borderRadius: borderRadius,
            ),
            alignment: Alignment.center,
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                if (isSelected) ...[
                  Icon(Icons.check, size: 16, color: colorScheme.primary),
                  const SizedBox(width: 6),
                ],
                Text(
                  text,
                  style: TextStyle(
                    color: isSelected
                        ? colorScheme.primary
                        : colorScheme.onSurface,
                    fontWeight: isSelected ? FontWeight.bold : FontWeight.w500,
                    fontSize: 13,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildEmptyState(ColorScheme colorScheme) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.notifications_none_outlined,
            size: 40,
            color: colorScheme.onSurfaceVariant,
          ),
          const SizedBox(height: 12),
          Text(
            'No tienes notificaciones',
            style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 15),
          ),
        ],
      ),
    );
  }

  Widget _buildNotificationsList(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return ListView.separated(
      itemCount: _notifications.length,
      separatorBuilder: (context, index) =>
          Divider(height: 1, color: appColors.divider),
      itemBuilder: (context, index) {
        return ListTile(
          leading: CircleAvatar(
            backgroundColor: appColors.selectedItem,
            child: Icon(Icons.notifications, color: colorScheme.primary),
          ),
          title: Text(
            _notifications[index],
            style: TextStyle(
              color: colorScheme.onSurface,
              fontWeight: FontWeight.w600,
            ),
          ),
          subtitle: Text(
            'Hace un momento',
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
        );
      },
    );
  }
}
