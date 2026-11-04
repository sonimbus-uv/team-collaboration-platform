import 'package:flutter/material.dart';

class NotificationsScreen extends StatefulWidget {
  const NotificationsScreen({super.key});

  @override
  State<NotificationsScreen> createState() => _NotificationsScreenState();
}

class _NotificationsScreenState extends State<NotificationsScreen> {
  // Estado para el ToggleGroup (0 = Todas, 1 = No leídas)
  int _selectedFilter = 0;
  final Color _primaryOrange = const Color(0xFFC44111);

  // Lista simulada para el historial de notificaciones
  final List<String> _notifications = [];

  void _handleMarkAllRead() {
    // TODO: Implementar lógica (CU-31) para marcar todo como leído
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Todas las notificaciones marcadas como leídas'),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFFF7F7F9),

      // La barra superior incluye el botón para retroceder automáticamente
      appBar: AppBar(
        backgroundColor: Colors.white,
        foregroundColor: Colors.black87,
        elevation: 1,
        title: const Text(
          'Notificaciones',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
        ),
        actions: [
          // En móvil, movemos el botón "Marcar leídas" al AppBar para ahorrar espacio
          TextButton.icon(
            onPressed: _handleMarkAllRead,
            icon: Icon(Icons.done_all, color: _primaryOrange, size: 20),
            label: Text(
              'Marcar leídas',
              style: TextStyle(
                color: _primaryOrange,
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
          // Equivalente al <HBox styleClass="filter-bar"> del FXML
          Container(
            color: Colors.white,
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
            child: Row(
              children: [
                _buildSegmentButton('Todas', 0, isLeft: true),
                _buildSegmentButton('No leídas (0)', 1, isLeft: false),
              ],
            ),
          ),
          const Divider(height: 1, color: Colors.black12),

          // ================= CONTENIDO CENTRAL =================
          // Equivalente al <StackPane> y <ListView> del FXML
          Expanded(
            child: _notifications.isEmpty
                ? _buildEmptyState()
                : _buildNotificationsList(),
          ),
        ],
      ),
    );
  }

  // Reproduce visualmente el "segment-left" y "segment-right" del escritorio
  Widget _buildSegmentButton(String text, int index, {required bool isLeft}) {
    final bool isSelected = _selectedFilter == index;

    return Expanded(
      child: InkWell(
        onTap: () => setState(() => _selectedFilter = index),
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 10),
          decoration: BoxDecoration(
            color: isSelected ? const Color(0xFFFFF5F2) : Colors.white,
            border: Border.all(
              color: isSelected ? _primaryOrange : Colors.black26,
            ),
            // Redondea solo las esquinas exteriores para simular un ToggleGroup unido
            borderRadius: isLeft
                ? const BorderRadius.only(
                    topLeft: Radius.circular(6),
                    bottomLeft: Radius.circular(6),
                  )
                : const BorderRadius.only(
                    topRight: Radius.circular(6),
                    bottomRight: Radius.circular(6),
                  ),
          ),
          alignment: Alignment.center,
          child: Text(
            text,
            style: TextStyle(
              color: isSelected ? _primaryOrange : Colors.black87,
              fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
              fontSize: 13,
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildEmptyState() {
    return const Center(
      child: Text(
        'No tienes notificaciones',
        style: TextStyle(color: Colors.black54, fontSize: 15),
      ),
    );
  }

  Widget _buildNotificationsList() {
    return ListView.separated(
      itemCount: _notifications.length,
      separatorBuilder: (context, index) => const Divider(height: 1),
      itemBuilder: (context, index) {
        return ListTile(
          leading: CircleAvatar(
            backgroundColor: Colors.grey.shade200,
            child: const Icon(Icons.notifications, color: Colors.black54),
          ),
          title: Text(_notifications[index]),
          subtitle: const Text('Hace un momento'),
        );
      },
    );
  }
}
