import 'package:flutter/material.dart';

import '../theme/app-theme-extension.dart';
import '../theme/theme-scope.dart';

class ProfileScreen extends StatefulWidget {
  const ProfileScreen({super.key});

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  // Controladores de texto
  final _nameController = TextEditingController(text: 'Luis Vázquez');
  final _currentPasswordController = TextEditingController();
  final _newPasswordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();

  // Estados visuales
  int _selectedAvatarIndex = 0;
  bool _obscureCurrentPwd = true;
  bool _obscureNewPwd = true;
  bool _obscureConfirmPwd = true;

  @override
  void dispose() {
    _nameController.dispose();
    _currentPasswordController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  void _showLogoutConfirmation() {
    final colorScheme = Theme.of(context).colorScheme;
    final appColors = Theme.of(context).extension<AppThemeColors>()!;

    showDialog(
      context: context,
      builder: (BuildContext dialogContext) {
        return AlertDialog(
          backgroundColor: colorScheme.surface,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(color: appColors.divider),
          ),
          title: Text(
            '¿Cerrar sesión?',
            style: TextStyle(
              fontWeight: FontWeight.bold,
              fontSize: 18,
              color: colorScheme.onSurface,
            ),
          ),
          content: Text(
            'Tendrás que volver a iniciar sesión para ver tus grupos y mensajes en este equipo.',
            style: TextStyle(color: colorScheme.onSurfaceVariant),
          ),
          actionsPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 12,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              style: TextButton.styleFrom(
                foregroundColor: colorScheme.onSurfaceVariant,
              ),
              child: const Text(
                'Cancelar',
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
            ),
            ElevatedButton(
              onPressed: () {
                // TODO: Ejecutar lógica de CU-06 Cerrar sesión
                Navigator.of(dialogContext)
                    .pushNamedAndRemoveUntil('/login', (route) => false);
              },
              style: ElevatedButton.styleFrom(
                backgroundColor: colorScheme.primary,
                foregroundColor: colorScheme.onPrimary,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(6),
                ),
                elevation: 0,
              ),
              child: const Text(
                'Cerrar sesión',
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
            ),
          ],
        );
      },
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
          'Mi perfil',
          style: TextStyle(
            fontSize: 18,
            fontWeight: FontWeight.bold,
            color: colorScheme.onSurface,
          ),
        ),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _buildAppearanceCard(colorScheme, appColors),
            const SizedBox(height: 20),
            _buildProfileDataCard(colorScheme, appColors),
            const SizedBox(height: 20),
            _buildChangePasswordCard(colorScheme, appColors),
            const SizedBox(height: 20),
            _buildSessionCard(colorScheme, appColors),
            const SizedBox(height: 32),
          ],
        ),
      ),
    );
  }

  // ================= TARJETA 0: APARIENCIA Y ACCESIBILIDAD =================
  Widget _buildAppearanceCard(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    final themeController = ThemeScope.of(context);
    final isDark = themeController.isDark;

    return Container(
      padding: const EdgeInsets.all(20),
      decoration: _cardDecoration(colorScheme, appColors),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Apariencia',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            secondary: Icon(
              isDark ? Icons.dark_mode_outlined : Icons.light_mode_outlined,
              color: colorScheme.primary,
            ),
            title: Text(
              'Modo oscuro',
              style: TextStyle(
                fontSize: 14,
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurface,
              ),
            ),
            subtitle: Text(
              isDark ? 'Activado' : 'Desactivado (modo claro)',
              style: TextStyle(
                color: colorScheme.onSurfaceVariant,
                fontSize: 12,
              ),
            ),
            value: isDark,
            activeThumbColor: colorScheme.onPrimary,
            activeTrackColor: colorScheme.primary,
            inactiveThumbColor: colorScheme.outline,
            inactiveTrackColor: appColors.highlight,
            onChanged: (enabled) {
              themeController.setThemeMode(
                enabled ? ThemeMode.dark : ThemeMode.light,
              );
            },
          ),
        ],
      ),
    );
  }

  // ================= TARJETA 1: DATOS DEL PERFIL =================
  Widget _buildProfileDataCard(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: _cardDecoration(colorScheme, appColors),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Datos del perfil',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 20),

          // Avatar Actual
          Container(
            width: 72,
            height: 72,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: appColors.selectedItem,
              border: Border.all(color: colorScheme.primary, width: 1.5),
            ),
            alignment: Alignment.center,
            child: Text(
              '?',
              style: TextStyle(
                color: colorScheme.primary,
                fontSize: 28,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
          const SizedBox(height: 24),

          // Campo Nombre
          _buildFieldLabel('Nombre', colorScheme),
          TextFormField(
            controller: _nameController,
            cursorColor: colorScheme.primary,
            style: TextStyle(color: colorScheme.onSurface),
            decoration: _inputDecoration(),
          ),
          const SizedBox(height: 20),

          // Selección de Avatar
          _buildFieldLabel('Foto de perfil', colorScheme),
          Text(
            'Elige una del paquete.',
            style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 12),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: List.generate(
              5,
              (index) => _buildAvatarOption(index, colorScheme, appColors),
            ),
          ),
          const SizedBox(height: 32),

          // Botones Guardar/Descartar
          Row(
            children: [
              ElevatedButton(
                onPressed: () {
                  // TODO: Lógica CU-07 Guardar perfil
                },
                style: _primaryButtonStyle(colorScheme),
                child: const Text('Guardar perfil'),
              ),
              const SizedBox(width: 12),
              OutlinedButton(
                onPressed: () {
                  _nameController.text = 'Luis Vázquez';
                  setState(() => _selectedAvatarIndex = 0);
                },
                style: _secondaryButtonStyle(colorScheme),
                child: const Text('Descartar'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  // ================= TARJETA 2: CAMBIAR CONTRASEÑA =================
  Widget _buildChangePasswordCard(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: _cardDecoration(colorScheme, appColors),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Cambiar contraseña',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 20),

          _buildFieldLabel('Contraseña actual', colorScheme),
          _buildPasswordField(
            _currentPasswordController,
            _obscureCurrentPwd,
            () => setState(() => _obscureCurrentPwd = !_obscureCurrentPwd),
            colorScheme,
          ),
          const SizedBox(height: 16),

          _buildFieldLabel('Nueva contraseña', colorScheme),
          _buildPasswordField(
            _newPasswordController,
            _obscureNewPwd,
            () => setState(() => _obscureNewPwd = !_obscureNewPwd),
            colorScheme,
          ),
          const SizedBox(height: 4),
          Text(
            'Mínimo 8 caracteres, con al menos una letra y un número.',
            style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 12),
          ),
          const SizedBox(height: 16),

          _buildFieldLabel('Confirmar nueva contraseña', colorScheme),
          _buildPasswordField(
            _confirmPasswordController,
            _obscureConfirmPwd,
            () => setState(() => _obscureConfirmPwd = !_obscureConfirmPwd),
            colorScheme,
          ),
          const SizedBox(height: 32),

          OutlinedButton(
            onPressed: () {
              // TODO: Lógica CU-07 Cambiar contraseña
            },
            style: _secondaryButtonStyle(colorScheme),
            child: const Text('Actualizar contraseña'),
          ),
        ],
      ),
    );
  }

  // ================= TARJETA 3: SESIÓN =================
  Widget _buildSessionCard(ColorScheme colorScheme, AppThemeColors appColors) {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: colorScheme.outline,
          style: BorderStyle.solid,
          width: 1.5,
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Sesión',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.bold,
                    color: colorScheme.onSurface,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  'Sales de Sonimbus en este equipo.',
                  style: TextStyle(
                    color: colorScheme.onSurfaceVariant,
                    fontSize: 13,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),
          ElevatedButton(
            onPressed: _showLogoutConfirmation,
            style: _primaryButtonStyle(colorScheme),
            child: const Text('Cerrar sesión'),
          ),
        ],
      ),
    );
  }

  // ================= COMPONENTES REUTILIZABLES =================

  BoxDecoration _cardDecoration(
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    return BoxDecoration(
      color: colorScheme.surface,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: appColors.divider),
    );
  }

  Widget _buildFieldLabel(String text, ColorScheme colorScheme) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8.0),
      child: Text(
        text,
        style: TextStyle(
          fontWeight: FontWeight.w700,
          fontSize: 13,
          color: colorScheme.onSurface,
        ),
      ),
    );
  }

  Widget _buildAvatarOption(
    int index,
    ColorScheme colorScheme,
    AppThemeColors appColors,
  ) {
    final isSelected = _selectedAvatarIndex == index;
    return Semantics(
      button: true,
      selected: isSelected,
      label: 'Avatar ${index + 1}',
      child: GestureDetector(
        onTap: () => setState(() => _selectedAvatarIndex = index),
        child: Container(
          width: 48,
          height: 48,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: isSelected ? appColors.selectedItem : appColors.highlight,
            border: Border.all(
              color: isSelected ? colorScheme.primary : colorScheme.outline,
              width: isSelected ? 2.5 : 1,
            ),
          ),
          alignment: Alignment.center,
          child: Icon(
            isSelected ? Icons.check : Icons.person_outline,
            color: isSelected
                ? colorScheme.primary
                : colorScheme.onSurfaceVariant,
          ),
        ),
      ),
    );
  }

  Widget _buildPasswordField(
    TextEditingController controller,
    bool isObscured,
    VoidCallback onToggle,
    ColorScheme colorScheme,
  ) {
    return TextFormField(
      controller: controller,
      obscureText: isObscured,
      cursorColor: colorScheme.primary,
      style: TextStyle(color: colorScheme.onSurface),
      decoration: _inputDecoration().copyWith(
        suffixIcon: IconButton(
          tooltip: isObscured ? 'Mostrar contraseña' : 'Ocultar contraseña',
          icon: Icon(
            isObscured
                ? Icons.visibility_off_outlined
                : Icons.visibility_outlined,
            color: colorScheme.onSurfaceVariant,
            size: 20,
          ),
          onPressed: onToggle,
        ),
      ),
    );
  }

  InputDecoration _inputDecoration() {
    return const InputDecoration(
      contentPadding: EdgeInsets.symmetric(horizontal: 14, vertical: 14),
    );
  }

  ButtonStyle _primaryButtonStyle(ColorScheme colorScheme) {
    return ElevatedButton.styleFrom(
      backgroundColor: colorScheme.primary,
      foregroundColor: colorScheme.onPrimary,
      elevation: 0,
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      textStyle: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
    );
  }

  ButtonStyle _secondaryButtonStyle(ColorScheme colorScheme) {
    return OutlinedButton.styleFrom(
      foregroundColor: colorScheme.onSurface,
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
      side: BorderSide(color: colorScheme.outline),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      textStyle: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
    );
  }
}
