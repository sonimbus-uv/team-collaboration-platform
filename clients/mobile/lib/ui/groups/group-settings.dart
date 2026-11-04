import 'package:flutter/material.dart';

class GroupSettingsScreen extends StatefulWidget {
  const GroupSettingsScreen({super.key});

  @override
  State<GroupSettingsScreen> createState() => _GroupSettingsScreenState();
}

class _GroupSettingsScreenState extends State<GroupSettingsScreen> {
  final Color _accentOrange = const Color(0xFFC44111);
  final Color _dangerRed = const Color(0xFFB3261E);

  // Controladores para la pestaña General
  final TextEditingController _nameController = TextEditingController(
    text: 'Sistemas Distribuidos',
  );
  final TextEditingController _descController = TextEditingController();

  // Controladores para otras pestañas
  final TextEditingController _newChannelController = TextEditingController();
  final TextEditingController _memberSearchController = TextEditingController();

  String _channelType = 'text';
  String _inviteMaxUses = '1 uso';
  String _inviteExpiration = '1 día';

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    _newChannelController.dispose();
    _memberSearchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 5,
      child: Scaffold(
        backgroundColor: Colors.white,
        appBar: AppBar(
          backgroundColor: Colors.white,
          elevation: 1,
          surfaceTintColor: Colors.transparent,
          leadingWidth: 160,
          leading: TextButton.icon(
            onPressed: () => Navigator.pop(context),
            icon: const Icon(Icons.arrow_back, color: Colors.black87, size: 20),
            label: const Text(
              'Volver al grupo',
              style: TextStyle(color: Colors.black87, fontSize: 14),
            ),
            style: TextButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              alignment: Alignment.centerLeft,
            ),
          ),
          title: const Text(
            'Configuración del grupo',
            style: TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.bold,
              color: Colors.black87,
            ),
          ),
          actions: [
            Padding(
              padding: const EdgeInsets.only(right: 16.0, top: 10, bottom: 10),
              child: OutlinedButton(
                onPressed: () {
                  // Lógica para abandonar grupo
                },
                style: OutlinedButton.styleFrom(
                  foregroundColor: Colors.black87,
                  side: const BorderSide(color: Colors.black26),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
                child: const Text('Abandonar grupo'),
              ),
            ),
          ],
          bottom: TabBar(
            isScrollable: true,
            labelColor: _accentOrange,
            unselectedLabelColor: Colors.black54,
            indicatorColor: _accentOrange,
            indicatorSize: TabBarIndicatorSize.tab,
            indicatorWeight: 3,
            tabs: const [
              Tab(text: 'General'),
              Tab(text: 'Invitaciones'),
              Tab(text: 'Canales'),
              Tab(text: 'Miembros y roles'),
              Tab(text: 'Moderación'),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            _buildGeneralTab(),
            _buildInvitationsTab(),
            _buildChannelsTab(),
            _buildMembersTab(),
            _buildModerationTab(),
          ],
        ),
      ),
    );
  }

  // ===================== PESTAÑA 1: GENERAL =====================
  Widget _buildGeneralTab() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Nombre del grupo',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _nameController,
            decoration: _inputDecoration('Nombre del grupo'),
          ),
          const SizedBox(height: 24),
          const Text(
            'Descripción',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _descController,
            maxLines: 4,
            maxLength: 200,
            decoration: _inputDecoration('¿Para qué es este grupo?'),
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              ElevatedButton(
                onPressed: () {},
                style: ElevatedButton.styleFrom(
                  backgroundColor: _accentOrange,
                  foregroundColor: Colors.white,
                  elevation: 0,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 24,
                    vertical: 16,
                  ),
                ),
                child: const Text(
                  'Guardar cambios',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
              OutlinedButton(
                onPressed: () {},
                style: OutlinedButton.styleFrom(
                  foregroundColor: Colors.black87,
                  side: const BorderSide(color: Colors.black26),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                  padding: const EdgeInsets.symmetric(
                    horizontal: 24,
                    vertical: 16,
                  ),
                ),
                child: const Text('Descartar'),
              ),
            ],
          ),
          const SizedBox(height: 48),

          // Zona de riesgo
          Container(
            padding: const EdgeInsets.all(24),
            decoration: BoxDecoration(
              border: Border.all(
                color: _dangerRed.withOpacity(0.5),
                width: 1.5,
                style: BorderStyle.solid,
              ),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'Zona de riesgo · solo administradores',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 16),
                ),
                const SizedBox(height: 24),
                _buildDangerZoneAction(
                  title: 'Archivar grupo',
                  description: 'El grupo queda en solo lectura. Puedes restaurarlo después.',
                  buttonText: 'Archivar',
                  isDanger: false,
                  onPressed: () {},
                ),
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 16.0),
                  child: Divider(color: Colors.black12),
                ),
                _buildDangerZoneAction(
                  title: 'Eliminar grupo',
                  description: 'Se borran canales, mensajes, archivos y proyectos. No se puede deshacer.',
                  buttonText: 'Eliminar grupo',
                  isDanger: true,
                  onPressed: () => _showDeleteGroupDialog(context),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildDangerZoneAction({
    required String title,
    required String description,
    required String buttonText,
    required bool isDanger,
    required VoidCallback onPressed,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
        const SizedBox(height: 4),
        Text(
          description,
          style: const TextStyle(color: Colors.black54, fontSize: 13),
        ),
        const SizedBox(height: 12),
        SizedBox(
          width: double.infinity,
          child: isDanger
              ? ElevatedButton(
                  onPressed: onPressed,
                  style: ElevatedButton.styleFrom(
                    backgroundColor: _dangerRed,
                    foregroundColor: Colors.white,
                    elevation: 0,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                  child: Text(buttonText),
                )
              : OutlinedButton(
                  onPressed: onPressed,
                  style: OutlinedButton.styleFrom(
                    foregroundColor: Colors.black87,
                    side: const BorderSide(color: Colors.black26),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                  child: Text(buttonText),
                ),
        ),
      ],
    );
  }

  // ===================== PESTAÑA 2: INVITACIONES =====================
  Widget _buildInvitationsTab() {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: Colors.grey[50],
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.black12),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _buildDropdownCol(
                  'Usos máximos',
                  ['1 uso', '5 usos', '10 usos', 'Sin límite'],
                  _inviteMaxUses,
                  (v) => setState(() => _inviteMaxUses = v!),
                ),
                const SizedBox(height: 16),
                _buildDropdownCol(
                  'Caduca en',
                  ['1 hora', '1 día', '7 días', 'Nunca'],
                  _inviteExpiration,
                  (v) => setState(() => _inviteExpiration = v!),
                ),
                const SizedBox(height: 24),
                ElevatedButton(
                  onPressed: () {},
                  style: ElevatedButton.styleFrom(
                    backgroundColor: _accentOrange,
                    foregroundColor: Colors.white,
                    elevation: 0,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const EdgeInsets.symmetric(vertical: 16),
                  ),
                  child: const Text(
                    'Generar código',
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          _buildMobileEmptyState(
            icon: Icons.mark_email_unread_outlined,
            message: 'Todavía no hay invitaciones',
          ),
        ],
      ),
    );
  }

  Widget _buildDropdownCol(
    String label,
    List<String> options,
    String currentValue,
    ValueChanged<String?> onChanged,
  ) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
        ),
        const SizedBox(height: 8),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          decoration: BoxDecoration(
            border: Border.all(color: Colors.black26),
            borderRadius: BorderRadius.circular(8),
            color: Colors.white,
          ),
          child: DropdownButtonHideUnderline(
            child: DropdownButton<String>(
              value: currentValue,
              isExpanded: true,
              items: options
                  .map((s) => DropdownMenuItem(value: s, child: Text(s)))
                  .toList(),
              onChanged: onChanged,
            ),
          ),
        ),
      ],
    );
  }

  // ===================== PESTAÑA 3: CANALES =====================
  Widget _buildChannelsTab() {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: Colors.grey[50],
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.black12),
            ),
            child: Column(
              // Cambiado a Column para adaptabilidad móvil
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const Text(
                  'Nombre del nuevo canal',
                  style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _newChannelController,
                  decoration: _inputDecoration('ej. entregas'),
                ),
                const SizedBox(height: 16),
                const Text(
                  'Tipo',
                  style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
                ),
                Row(
                  children: [
                    Radio<String>(
                      value: 'text',
                      groupValue: _channelType,
                      activeColor: _accentOrange,
                      onChanged: (v) => setState(() => _channelType = v!),
                    ),
                    const Text('Texto'),
                    const SizedBox(width: 16),
                    Radio<String>(
                      value: 'voice',
                      groupValue: _channelType,
                      activeColor: _accentOrange,
                      onChanged: (v) => setState(() => _channelType = v!),
                    ),
                    const Text('Voz'),
                  ],
                ),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: () {},
                  style: ElevatedButton.styleFrom(
                    backgroundColor: _accentOrange,
                    foregroundColor: Colors.white,
                    elevation: 0,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    padding: const EdgeInsets.symmetric(vertical: 16),
                  ),
                  child: const Text(
                    'Crear canal',
                    style: TextStyle(fontWeight: FontWeight.bold),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),
          const Text(
            'CANALES DE TEXTO (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: Colors.black54,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(child: _buildEmptyListContainer()),
          const SizedBox(height: 24),
          const Text(
            'CANALES DE VOZ (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: Colors.black54,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(child: _buildEmptyListContainer()),
        ],
      ),
    );
  }

  Widget _buildEmptyListContainer() {
    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        border: Border.all(color: Colors.black12),
        borderRadius: BorderRadius.circular(8),
      ),
    );
  }

  // ===================== PESTAÑA 4: MIEMBROS Y ROLES =====================
  Widget _buildMembersTab() {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment:
            CrossAxisAlignment.stretch, // Expande los hijos al máximo ancho
        children: [
          const Text(
            'MIEMBROS (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: Colors.black54,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 16),
          const Text(
            'Buscar miembro',
            style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
          ), // Tamaño de fuente ajustado
          const SizedBox(height: 8),
          TextField(
            controller: _memberSearchController,
            decoration: _inputDecoration('Nombre o correo'),
          ),
          const SizedBox(height: 24),
          // Reemplazo de tabla horizontal por estado vacío adaptado a móvil
          _buildMobileEmptyState(
            icon: Icons.group_off_outlined,
            message: 'Sin miembros que mostrar',
          ),
          const SizedBox(height: 16),
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Text(
                'Mostrando 0 de 0',
                style: TextStyle(color: Colors.black54, fontSize: 13),
              ),
              const SizedBox(width: 8),
              InkWell(
                onTap: () {},
                child: Text(
                  'Ver todos',
                  style: TextStyle(
                    color: _accentOrange,
                    decoration: TextDecoration.underline,
                    fontSize: 13,
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  // ===================== PESTAÑA 5: MODERACIÓN =====================
  Widget _buildModerationTab() {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'BLOQUEADOS (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: Colors.black54,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 8),
          const Text(
            'Un miembro bloqueado no puede volver a unirse con una invitación. Para expulsar o bloquear a alguien usa la pestaña «Miembros y roles».',
            style: TextStyle(color: Colors.black54, fontSize: 14),
          ),
          const SizedBox(height: 24),
          _buildMobileEmptyState(
            icon: Icons.block,
            message: 'No hay miembros bloqueados',
          ),
        ],
      ),
    );
  }

  // ===================== UTILIDADES =====================
  InputDecoration _inputDecoration(String hint) {
    return InputDecoration(
      hintText: hint,
      hintStyle: const TextStyle(color: Colors.black38),
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: const BorderSide(color: Colors.black26),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: _accentOrange, width: 2),
      ),
    );
  }

  // Concepto de Usabilidad Móvil: En lugar de renderizar encabezados de tabla (que se aplastan)
  // mostramos un diseño de tarjeta vacío. Cuando haya datos, se recomienda usar ListView con Card o ListTile.
  Widget _buildMobileEmptyState({
    required IconData icon,
    required String message,
  }) {
    return Expanded(
      child: Container(
        width: double.infinity,
        decoration: BoxDecoration(
          color: const Color(0xFFFAFAFA),
          border: Border.all(color: Colors.black12),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 40, color: Colors.black26),
            const SizedBox(height: 12),
            Text(
              message,
              style: const TextStyle(color: Colors.black54, fontSize: 14),
            ),
          ],
        ),
      ),
    );
  }

  // ===================== DIÁLOGO ELIMINAR =====================
  void _showDeleteGroupDialog(BuildContext context) {
    final TextEditingController confirmController = TextEditingController();

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        backgroundColor: Colors.white,
        surfaceTintColor: Colors.transparent,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        title: const Text(
          '¿Eliminar el grupo?',
          style: TextStyle(fontWeight: FontWeight.bold, fontSize: 20),
        ),
        content: SizedBox(
          width: MediaQuery.of(context).size.width * 0.9,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'Se borrarán de forma permanente todos los canales, mensajes, archivos y proyectos del grupo, y sus miembros perderán el acceso. Esta acción no se puede deshacer.',
                style: TextStyle(color: Colors.black87, fontSize: 14),
              ),
              const SizedBox(height: 24),
              const Text(
                'Escribe el nombre del grupo para confirmar',
                style: TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: confirmController,
                decoration: _inputDecoration(''),
              ),
              const SizedBox(height: 8),
              const Text(
                'El botón se activa cuando el nombre coincide exactamente.',
                style: TextStyle(color: Colors.black54, fontSize: 12),
              ),
            ],
          ),
        ),
        actionsPadding: const EdgeInsets.symmetric(
          horizontal: 24,
          vertical: 20,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            style: TextButton.styleFrom(foregroundColor: Colors.black54),
            child: const Text('Cancelar'),
          ),
          ElevatedButton(
            onPressed: null, // Habilitar cuando confirmController.text coincida
            style: ElevatedButton.styleFrom(
              backgroundColor: _dangerRed,
              disabledBackgroundColor: _dangerRed.withOpacity(0.5),
              foregroundColor: Colors.white,
              disabledForegroundColor: Colors.white,
              elevation: 0,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
              ),
            ),
            child: const Text(
              'Eliminar grupo',
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}
