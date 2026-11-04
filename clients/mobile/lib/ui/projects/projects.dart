import 'package:flutter/material.dart';

class ProjectsAndTasksScreen extends StatefulWidget {
  const ProjectsAndTasksScreen({Key? key}) : super(key: key);

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
    return Scaffold(
      backgroundColor: Colors.white,
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 0,
        iconTheme: const IconThemeData(color: Colors.black87),
        title: const Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Proyectos y tareas',
              style: TextStyle(
                fontSize: 18,
                fontWeight: FontWeight.bold,
                color: Colors.black87,
              ),
            ),
            Text(
              'Selecciona un proyecto',
              style: TextStyle(
                fontSize: 13,
                fontWeight: FontWeight.normal,
                color: Colors.black54,
              ),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.filter_list),
            tooltip: 'Filtrar responsable',
            onPressed: _showFilterDialog,
          ),
          // Botón '+' múltiple para elegir entre nuevo proyecto o nueva tarea
          PopupMenuButton<String>(
            icon: const Icon(Icons.add),
            tooltip: 'Crear nuevo',
            offset: const Offset(0, 45),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
            ),
            onSelected: (String value) {
              if (value == 'project') {
                // TODO: Implementar lógica de creación de proyecto acorde al MVP
              } else if (value == 'task') {
                // TODO: Implementar lógica de creación de tarea acorde al MVP
              }
            },
            itemBuilder: (BuildContext context) => <PopupMenuEntry<String>>[
              const PopupMenuItem<String>(
                value: 'project',
                child: ListTile(
                  leading: Icon(Icons.folder_open_outlined, size: 22),
                  title: Text('Nuevo proyecto', style: TextStyle(fontSize: 15)),
                  contentPadding: EdgeInsets.zero,
                  visualDensity: VisualDensity.compact,
                ),
              ),
              const PopupMenuItem<String>(
                value: 'task',
                child: ListTile(
                  leading: Icon(Icons.check_circle_outline, size: 22),
                  title: Text('Nueva tarea', style: TextStyle(fontSize: 15)),
                  contentPadding: EdgeInsets.zero,
                  visualDensity: VisualDensity.compact,
                ),
              ),
            ],
          ),
          const SizedBox(width: 8),
        ],
      ),
      drawer: _buildProjectsDrawer(),
      body: _buildKanbanBoard(),
    );
  }

  Widget _buildProjectsDrawer() {
    return Drawer(
      backgroundColor: Colors.white,
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Padding(
              padding: EdgeInsets.fromLTRB(16, 24, 16, 12),
              child: Text(
                'PROYECTOS (0)',
                style: TextStyle(
                  fontWeight: FontWeight.bold,
                  color: Colors.black54,
                  fontSize: 13,
                ),
              ),
            ),
            const Divider(height: 1),
            Expanded(
              child: ListView(
                padding: EdgeInsets.zero,
                children: const [
                  ListTile(
                    title: Text(
                      'No hay proyectos',
                      style: TextStyle(color: Colors.black38),
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

  Widget _buildKanbanBoard() {
    // Modificado a scroll vertical para mostrar los paneles en forma de lista apilada
    return ListView(
      scrollDirection: Axis.vertical,
      padding: const EdgeInsets.all(16.0),
      children: [
        _buildKanbanColumn(
          title: 'Pendiente',
          count: 0,
          icon: Icons.circle_outlined,
          iconColor: Colors.black54,
          backgroundColor: const Color(0xFFF4F5F7), // Gris claro pastel
          chipBorderColor: Colors.black12,
        ),
        const SizedBox(height: 16),
        _buildKanbanColumn(
          title: 'En progreso',
          count: 0,
          icon: Icons.access_time,
          iconColor: Colors.deepOrange.shade400,
          backgroundColor: const Color(0xFFFFF7E6), // Naranja/Amarillo pastel
          chipBorderColor: Colors.orange.shade200,
        ),
        const SizedBox(height: 16),
        _buildKanbanColumn(
          title: 'Completada',
          count: 0,
          icon: Icons.check_circle_outline,
          iconColor: Colors.green.shade600,
          backgroundColor: const Color(0xFFEAF5EA), // Verde pastel
          chipBorderColor: Colors.green.shade200,
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
    required Color chipBorderColor,
  }) {
    return Container(
      width: double.infinity, // Se adapta al ancho total de la pantalla móvil
      height: 190, // Altura fija ideal para visualizar secciones limpias en lista vertical
      decoration: BoxDecoration(
        color: backgroundColor,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: Colors.black.withOpacity(0.05)),
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
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(20),
                    border: Border.all(color: chipBorderColor),
                    boxShadow: [
                      BoxShadow(
                        color: Colors.black.withOpacity(0.02),
                        blurRadius: 2,
                        offset: const Offset(0, 1),
                      ),
                    ],
                  ),
                  child: Row(
                    children: [
                      Icon(icon, color: iconColor, size: 16),
                      const SizedBox(width: 6),
                      Text(
                        title,
                        style: const TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 13,
                          color: Colors.black87,
                        ),
                      ),
                    ],
                  ),
                ),
                const Spacer(),
                Text(
                  '$count',
                  style: const TextStyle(
                    fontWeight: FontWeight.bold,
                    color: Colors.black54,
                    fontSize: 15,
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

  void _showFilterDialog() {
    showDialog(
      context: context,
      builder: (context) {
        return AlertDialog(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          title: const Text(
            'Responsable',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
          ),
          content: DropdownButtonFormField<String>(
            value: _selectedAssignee,
            decoration: InputDecoration(
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
              ),
              contentPadding: const EdgeInsets.symmetric(
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
                Navigator.pop(context);
              }
            },
          ),
        );
      },
    );
  }
}
