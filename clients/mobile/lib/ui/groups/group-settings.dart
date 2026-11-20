import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';

class GroupSettingsScreen extends StatefulWidget {
  const GroupSettingsScreen({super.key});

  @override
  State<GroupSettingsScreen> createState() => _GroupSettingsScreenState();
}

class _GroupSettingsScreenState extends State<GroupSettingsScreen> {
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
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final appColors = theme.extension<AppThemeColors>()!;

    return DefaultTabController(
      length: 5,
      child: Scaffold(
        appBar: AppBar(
          backgroundColor: colorScheme.surface,
          foregroundColor: colorScheme.onSurface,
          elevation: 0,
          surfaceTintColor: Colors.transparent,
          leadingWidth: 160,
          leading: TextButton.icon(
            onPressed: () => Navigator.pop(context),
            icon: Icon(
              Icons.arrow_back,
              color: colorScheme.onSurface,
              size: 20,
            ),
            label: Text(
              'Volver al grupo',
              style: TextStyle(color: colorScheme.onSurface, fontSize: 14),
            ),
            style: TextButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              alignment: Alignment.centerLeft,
            ),
          ),
          title: Text(
            'Configuración del grupo',
            style: TextStyle(
              fontSize: 18,
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
          ),
          actions: [
            Padding(
              padding: const EdgeInsets.only(right: 16.0, top: 10, bottom: 10),
              child: OutlinedButton(
                onPressed: () {
                  // TODO: Lógica para abandonar grupo
                },
                style: OutlinedButton.styleFrom(
                  foregroundColor: colorScheme.onSurface,
                  side: BorderSide(color: colorScheme.outline),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
                child: const Text('Abandonar grupo'),
              ),
            ),
          ],
          bottom: PreferredSize(
            preferredSize: const Size.fromHeight(kTextTabBarHeight + 1),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                TabBar(
                  isScrollable: true,
                  tabAlignment: TabAlignment.start,
                  labelColor: colorScheme.primary,
                  labelStyle: const TextStyle(fontWeight: FontWeight.bold),
                  unselectedLabelColor: colorScheme.onSurfaceVariant,
                  unselectedLabelStyle: const TextStyle(
                    fontWeight: FontWeight.normal,
                  ),
                  indicatorColor: colorScheme.primary,
                  indicatorSize: TabBarIndicatorSize.tab,
                  indicatorWeight: 3,
                  dividerColor: Colors.transparent,
                  tabs: const [
                    Tab(text: 'General'),
                    Tab(text: 'Invitaciones'),
                    Tab(text: 'Canales'),
                    Tab(text: 'Miembros y roles'),
                    Tab(text: 'Moderación'),
                  ],
                ),
                Divider(height: 1, color: appColors.divider),
              ],
            ),
          ),
        ),
        body: TabBarView(
          children: [
            _buildGeneralTab(colorScheme, appColors),
            _buildInvitationsTab(colorScheme, appColors),
            _buildChannelsTab(colorScheme, appColors),
            _buildMembersTab(colorScheme, appColors),
            _buildModerationTab(colorScheme, appColors),
          ],
        ),
      ),
    );
  }

  // ===================== PESTAÑA 1: GENERAL =====================
  Widget _buildGeneralTab(ColorScheme colorScheme, AppThemeColors appColors) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Nombre del grupo',
            style: TextStyle(
              fontWeight: FontWeight.w600,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _nameController,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _inputDecoration('Nombre del grupo'),
          ),
          const SizedBox(height: 24),
          Text(
            'Descripción',
            style: TextStyle(
              fontWeight: FontWeight.w600,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _descController,
            maxLines: 4,
            maxLength: 200,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _inputDecoration('¿Para qué es este grupo?'),
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              ElevatedButton(
                onPressed: () {
                  // TODO: Guardar cambios generales del grupo
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
                    vertical: 16,
                  ),
                ),
                child: const Text(
                  'Guardar cambios',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
              ),
              OutlinedButton(
                onPressed: () {
                  // TODO: Descartar cambios generales del grupo
                },
                style: OutlinedButton.styleFrom(
                  foregroundColor: colorScheme.onSurface,
                  side: BorderSide(color: colorScheme.outline),
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
              color: appColors.errorBackground.withValues(alpha: 0.35),
              border: Border.all(
                color: colorScheme.error.withValues(alpha: 0.6),
                width: 1.5,
              ),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.warning_amber_rounded,
                      color: colorScheme.error,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'Zona de riesgo · solo administradores',
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          fontSize: 16,
                          color: colorScheme.onSurface,
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 24),
                _buildDangerZoneAction(
                  title: 'Archivar grupo',
                  description: 'El grupo queda en solo lectura. Puedes restaurarlo después.',
                  buttonText: 'Archivar',
                  isDanger: false,
                  colorScheme: colorScheme,
                  onPressed: () {
                    // TODO: Lógica para archivar grupo
                  },
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(vertical: 16.0),
                  child: Divider(color: appColors.divider),
                ),
                _buildDangerZoneAction(
                  title: 'Eliminar grupo',
                  description: 'Se borran canales, mensajes, archivos y proyectos. No se puede deshacer.',
                  buttonText: 'Eliminar grupo',
                  isDanger: true,
                  colorScheme: colorScheme,
                  onPressed: () =>
                      _showDeleteGroupDialog(context, colorScheme, appColors),
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
    required ColorScheme colorScheme,
    required VoidCallback onPressed,
  }) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          style: TextStyle(
            fontWeight: FontWeight.w600,
            color: colorScheme.onSurface,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          description,
          style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 13),
        ),
        const SizedBox(height: 12),
        SizedBox(
          width: double.infinity,
          child: isDanger
              ? ElevatedButton(
                  onPressed: onPressed,
                  style: ElevatedButton.styleFrom(
                    backgroundColor: colorScheme.error,
                    foregroundColor: colorScheme.onError,
                    elevation: 0,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                  child: Text(
                    buttonText,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                  ),
                )
              : OutlinedButton(
                  onPressed: onPressed,
                  style: OutlinedButton.styleFrom(
                    foregroundColor: colorScheme.onSurface,
                    side: BorderSide(color: colorScheme.outline),
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
  Widget _buildInvitationsTab(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: colorScheme.surface,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: appColors.divider),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                _buildDropdownCol(
                  'Usos máximos',
                  ['1 uso', '5 usos', '10 usos', 'Sin límite'],
                  _inviteMaxUses,
                  (v) => setState(() => _inviteMaxUses = v!),
                  colorScheme,
                ),
                const SizedBox(height: 16),
                _buildDropdownCol(
                  'Caduca en',
                  ['1 hora', '1 día', '7 días', 'Nunca'],
                  _inviteExpiration,
                  (v) => setState(() => _inviteExpiration = v!),
                  colorScheme,
                ),
                const SizedBox(height: 24),
                ElevatedButton(
                  onPressed: () {
                    // TODO: Generar código de invitación
                  },
                  style: ElevatedButton.styleFrom(
                    backgroundColor: colorScheme.primary,
                    foregroundColor: colorScheme.onPrimary,
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
            colorScheme: colorScheme,
            appColors: appColors,
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
        const SizedBox(height: 8),
        Container(
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
              iconEnabledColor: colorScheme.onSurfaceVariant,
              style: TextStyle(color: colorScheme.onSurface, fontSize: 14),
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
  Widget _buildChannelsTab(ColorScheme colorScheme, AppThemeColors appColors) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            padding: const EdgeInsets.all(20),
            decoration: BoxDecoration(
              color: colorScheme.surface,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: appColors.divider),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'Nombre del nuevo canal',
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 13,
                    color: colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _newChannelController,
                  cursorColor: colorScheme.primary,
                  style: TextStyle(color: colorScheme.onSurface),
                  decoration: _inputDecoration('ej. entregas'),
                ),
                const SizedBox(height: 16),
                Text(
                  'Tipo',
                  style: TextStyle(
                    fontWeight: FontWeight.w600,
                    fontSize: 13,
                    color: colorScheme.onSurface,
                  ),
                ),
                Row(
                  children: [
                    Radio<String>(
                      value: 'text',
                      groupValue: _channelType,
                      activeColor: colorScheme.primary,
                      onChanged: (v) => setState(() => _channelType = v!),
                    ),
                    Text(
                      'Texto',
                      style: TextStyle(color: colorScheme.onSurface),
                    ),
                    const SizedBox(width: 16),
                    Radio<String>(
                      value: 'voice',
                      groupValue: _channelType,
                      activeColor: colorScheme.primary,
                      onChanged: (v) => setState(() => _channelType = v!),
                    ),
                    Text('Voz', style: TextStyle(color: colorScheme.onSurface)),
                  ],
                ),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: () {
                    // TODO: Crear nuevo canal
                  },
                  style: ElevatedButton.styleFrom(
                    backgroundColor: colorScheme.primary,
                    foregroundColor: colorScheme.onPrimary,
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
          Text(
            'CANALES DE TEXTO (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurfaceVariant,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(child: _buildEmptyListContainer(colorScheme, appColors)),
          const SizedBox(height: 24),
          Text(
            'CANALES DE VOZ (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurfaceVariant,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 8),
          Expanded(child: _buildEmptyListContainer(colorScheme, appColors)),
        ],
      ),
    );
  }

  Widget _buildEmptyListContainer(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: colorScheme.surface,
        border: Border.all(color: appColors.divider),
        borderRadius: BorderRadius.circular(8),
      ),
    );
  }

  // ===================== PESTAÑA 4: MIEMBROS Y ROLES =====================
  Widget _buildMembersTab(ColorScheme colorScheme, AppThemeColors appColors) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'MIEMBROS (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurfaceVariant,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            'Buscar miembro',
            style: TextStyle(
              fontWeight: FontWeight.w600,
              fontSize: 13,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _memberSearchController,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _inputDecoration('Nombre o correo'),
          ),
          const SizedBox(height: 24),
          _buildMobileEmptyState(
            icon: Icons.group_off_outlined,
            message: 'Sin miembros que mostrar',
            colorScheme: colorScheme,
            appColors: appColors,
          ),
          const SizedBox(height: 16),
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(
                'Mostrando 0 de 0',
                style: TextStyle(
                  color: colorScheme.onSurfaceVariant,
                  fontSize: 13,
                ),
              ),
              const SizedBox(width: 8),
              InkWell(
                onTap: () {},
                child: Text(
                  'Ver todos',
                  style: TextStyle(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w600,
                    decoration: TextDecoration.underline,
                    decorationColor: colorScheme.primary,
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
  Widget _buildModerationTab(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Padding(
      padding: const EdgeInsets.all(24.0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'BLOQUEADOS (0)',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurfaceVariant,
              fontSize: 12,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Un miembro bloqueado no puede volver a unirse con una invitación. Para expulsar o bloquear a alguien usa la pestaña «Miembros y roles».',
            style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 14),
          ),
          const SizedBox(height: 24),
          _buildMobileEmptyState(
            icon: Icons.block,
            message: 'No hay miembros bloqueados',
            colorScheme: colorScheme,
            appColors: appColors,
          ),
        ],
      ),
    );
  }

  // ===================== UTILIDADES =====================
  InputDecoration _inputDecoration(String hint) {
    return InputDecoration(
      hintText: hint.isEmpty ? null : hint,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
    );
  }

  Widget _buildMobileEmptyState({
    required IconData icon,
    required String message,
    required ColorScheme colorScheme,
    required AppThemeColors appColors,
  }) {
    return Expanded(
      child: Container(
        width: double.infinity,
        decoration: BoxDecoration(
          color: colorScheme.surface,
          border: Border.all(color: appColors.divider),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 40, color: colorScheme.onSurfaceVariant),
            const SizedBox(height: 12),
            Text(
              message,
              style: TextStyle(
                color: colorScheme.onSurfaceVariant,
                fontSize: 14,
              ),
            ),
          ],
        ),
      ),
    );
  }

  // ===================== DIÁLOGO ELIMINAR =====================
  void _showDeleteGroupDialog(
    BuildContext context,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    final TextEditingController confirmController = TextEditingController();

    showDialog(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (context, setDialogState) {
          final isMatch =
              confirmController.text.trim() == _nameController.text.trim() &&
              _nameController.text.trim().isNotEmpty;

          return AlertDialog(
            backgroundColor: colorScheme.surface,
            surfaceTintColor: Colors.transparent,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
              side: BorderSide(color: appColors.divider),
            ),
            title: Text(
              '¿Eliminar el grupo?',
              style: TextStyle(
                fontWeight: FontWeight.bold,
                fontSize: 20,
                color: colorScheme.onSurface,
              ),
            ),
            content: SizedBox(
              width: MediaQuery.of(context).size.width * 0.9,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Se borrarán de forma permanente todos los canales, mensajes, archivos y proyectos del grupo, y sus miembros perderán el acceso. Esta acción no se puede deshacer.',
                    style: TextStyle(
                      color: colorScheme.onSurfaceVariant,
                      fontSize: 14,
                    ),
                  ),
                  const SizedBox(height: 24),
                  Text(
                    'Escribe el nombre del grupo para confirmar',
                    style: TextStyle(
                      fontWeight: FontWeight.w600,
                      fontSize: 13,
                      color: colorScheme.onSurface,
                    ),
                  ),
                  const SizedBox(height: 8),
                  TextField(
                    controller: confirmController,
                    cursorColor: colorScheme.error,
                    style: TextStyle(color: colorScheme.onSurface),
                    onChanged: (_) => setDialogState(() {}),
                    decoration: _inputDecoration(''),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'El botón se activa cuando el nombre coincide exactamente.',
                    style: TextStyle(
                      color: colorScheme.onSurfaceVariant,
                      fontSize: 12,
                    ),
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
                onPressed: () => Navigator.pop(dialogContext),
                style: TextButton.styleFrom(
                  foregroundColor: colorScheme.onSurfaceVariant,
                ),
                child: const Text('Cancelar'),
              ),
              ElevatedButton(
                onPressed: isMatch
                    ? () {
                        // TODO: Ejecutar eliminación permanente del grupo
                        Navigator.pop(dialogContext);
                      }
                    : null,
                style: ElevatedButton.styleFrom(
                  backgroundColor: colorScheme.error,
                  disabledBackgroundColor: colorScheme.error.withValues(
                    alpha: 0.35,
                  ),
                  foregroundColor: colorScheme.onError,
                  disabledForegroundColor: colorScheme.onError.withValues(
                    alpha: 0.75,
                  ),
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
          );
        },
      ),
    ).whenComplete(confirmController.dispose);
  }
}
