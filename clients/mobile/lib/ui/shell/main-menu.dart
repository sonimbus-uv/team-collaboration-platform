import 'package:flutter/material.dart';
import 'package:mobile/ui/groups/join-group.dart';
import 'package:mobile/ui/groups/create-group.dart';
import 'package:mobile/ui/files/shared-files.dart'; // <-- Nueva importación
import 'package:mobile/ui/projects/projects.dart'; // <-- Nueva importación
import 'package:mobile/ui/reports/reports.dart'; // <-- Nueva importación

class MainShellScreen extends StatefulWidget {
  const MainShellScreen({super.key});

  @override
  State<MainShellScreen> createState() => _MainShellScreenState();
}

class _MainShellScreenState extends State<MainShellScreen> {
  int _currentIndex = 0;

  // Colores extraídos de la interfaz de escritorio
  final Color _sidebarColor = const Color(0xFF381F38);
  final Color _textMuted = const Color(0xFFE4D9E5);
  final Color _accentOrange = const Color(0xFFC44111);

  // Vistas controladas por el menú lateral
  final List<Widget> _views = [
    const Center(
      child: Text(
        'Selecciona un canal para empezar',
        style: TextStyle(color: Colors.black54, fontSize: 15),
      ),
    ),
    const SharedFilesScreen(), // <-- Vista de Archivos Compartidos integrada
    const ProjectsAndTasksScreen(), // <-- Vista de Proyectos y tareas integrada
    const ReportsScreen(), // <-- Vista de Reportes integrada
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.white,
      appBar: AppBar(
        backgroundColor: Colors.white,
        elevation: 1,
        iconTheme: const IconThemeData(color: Colors.black87),
        actions: [
          IconButton(
            icon: const Icon(Icons.notifications_none, size: 26),
            onPressed: () => Navigator.pushNamed(context, '/notifications'),
          ),
          Padding(
            padding: const EdgeInsets.only(right: 16.0, left: 8.0),
            child: GestureDetector(
              onTap: () => Navigator.pushNamed(context, '/profile'),
              child: Container(
                width: 32,
                height: 32,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  border: Border.all(color: _accentOrange, width: 1.5),
                  color: const Color(0xFFFFF5F2),
                ),
                alignment: Alignment.center,
                child: Text(
                  '?',
                  style: TextStyle(
                    color: _accentOrange,
                    fontWeight: FontWeight.bold,
                    fontSize: 16,
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
      drawer: _buildDrawer(),
      body: IndexedStack(index: _currentIndex, children: _views),
    );
  }

  Widget _buildDrawer() {
    return Drawer(
      backgroundColor: _sidebarColor,
      child: SafeArea(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 20, 16, 20),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text(
                    'Selecciona un grupo',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 15,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  GestureDetector(
                    onTap: () {
                      Navigator.pushNamed(context, '/group-settings');
                    },
                    child: Container(
                      decoration: BoxDecoration(
                        border: Border.all(color: Colors.white38),
                        borderRadius: BorderRadius.circular(6),
                      ),
                      padding: const EdgeInsets.all(4),
                      child: const Icon(
                        Icons.settings_outlined,
                        color: Colors.white,
                        size: 20,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const Divider(color: Colors.white12, height: 1),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(vertical: 16),
                children: [
                  _buildSectionHeader('CANALES DE TEXTO'),
                  const SizedBox(height: 32),
                  _buildSectionHeader('CANALES DE VOZ'),
                  const SizedBox(height: 48),
                  const Padding(
                    padding: EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                    child: Text(
                      'GRUPO',
                      style: TextStyle(
                        color: Colors.white,
                        fontSize: 12,
                        fontWeight: FontWeight.bold,
                        letterSpacing: 0.5,
                      ),
                    ),
                  ),
                  _buildSidebarItem('Archivos compartidos', 1),
                  _buildSidebarItem('Proyectos y tareas', 2),
                  _buildSidebarItem('Reportes', 3),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(20.0),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Container(
                  width: 46,
                  height: 46,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    border: Border.all(color: Colors.white24, width: 1.5),
                  ),
                  child: IconButton(
                    icon: const Icon(
                      Icons.add,
                      color: Colors.white54,
                      size: 22,
                    ),
                    onPressed: () {
                      _showGroupOptionsModal(context);
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

  void _showGroupOptionsModal(BuildContext context) {
    showModalBottomSheet(
      context: context,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (BuildContext context) {
        return SafeArea(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(height: 8),
              Container(
                width: 40,
                height: 4,
                decoration: BoxDecoration(
                  color: Colors.grey[300],
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(height: 16),
              ListTile(
                leading: Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: const Color(0xFFFFF5F2),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Icon(Icons.group_add_outlined, color: _accentOrange),
                ),
                title: const Text(
                  'Crear grupo',
                  style: TextStyle(fontWeight: FontWeight.w600),
                ),
                subtitle: const Text('Inicia una nueva comunidad'),
                onTap: () {
                  Navigator.pop(context);
                  showDialog(
                    context: context,
                    builder: (BuildContext context) =>
                        const CreateGroupDialog(),
                  );
                },
              ),
              ListTile(
                leading: Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: Colors.grey[100],
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Icon(
                    Icons.login_outlined,
                    color: Colors.black54,
                  ),
                ),
                title: const Text(
                  'Unirse a un grupo',
                  style: TextStyle(fontWeight: FontWeight.w600),
                ),
                subtitle: const Text('Ingresa con un código de invitación'),
                onTap: () {
                  Navigator.pop(context);
                  showDialog(
                    context: context,
                    builder: (BuildContext context) => const JoinGroupDialog(),
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

  Widget _buildSectionHeader(String title) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            title,
            style: const TextStyle(
              color: Colors.white,
              fontSize: 12,
              fontWeight: FontWeight.bold,
              letterSpacing: 0.5,
            ),
          ),
          const Icon(Icons.add, color: Colors.white54, size: 16),
        ],
      ),
    );
  }

  Widget _buildSidebarItem(String title, int index) {
    return InkWell(
      onTap: () {
        setState(() => _currentIndex = index);
        Navigator.pop(context);
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
        child: Text(
          title,
          style: TextStyle(
            color: _currentIndex == index ? Colors.white : _textMuted,
            fontSize: 14,
            fontWeight: _currentIndex == index
                ? FontWeight.w600
                : FontWeight.normal,
          ),
        ),
      ),
    );
  }
}
