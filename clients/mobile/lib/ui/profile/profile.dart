import 'package:flutter/material.dart';

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

  // Estados visuales (simulando los fx:id del FXML)
  int _selectedAvatarIndex = 0;
  bool _obscureCurrentPwd = true;
  bool _obscureNewPwd = true;
  bool _obscureConfirmPwd = true;

  // Colores de la paleta
  final Color _primaryOrange = const Color(0xFFC44111);
  final Color _bgLight = const Color(0xFFF7F7F9);
  final Color _cardBorder = const Color(0xFFE0E0E0);

  @override
  void dispose() {
    _nameController.dispose();
    _currentPasswordController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  void _showLogoutConfirmation() {
    showDialog(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          title: const Text(
            '¿Cerrar sesión?',
            style: TextStyle(fontWeight: FontWeight.bold, fontSize: 18),
          ),
          content: const Text(
            'Tendrás que volver a iniciar sesión para ver tus grupos y mensajes en este equipo.',
            style: TextStyle(color: Colors.black87),
          ),
          actionsPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 12,
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(), // Cancelar
              style: TextButton.styleFrom(foregroundColor: Colors.black54),
              child: const Text(
                'Cancelar',
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
            ),
            ElevatedButton(
              onPressed: () {
                // TODO: Ejecutar lógica de CU-06 Cerrar sesión
                Navigator.of(context)
                    .pushNamedAndRemoveUntil('/login', (route) => false);
              },
              style: ElevatedButton.styleFrom(
                backgroundColor: Colors.black87,
                foregroundColor: Colors.white,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(6),
                ),
                elevation: 0,
              ),
              child: const Text('Cerrar sesión'),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: _bgLight,
      appBar: AppBar(
        backgroundColor: Colors.white,
        foregroundColor: Colors.black87,
        elevation: 1,
        title: const Text(
          'Mi perfil',
          style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
        ),
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            _buildProfileDataCard(),
            const SizedBox(height: 20),
            _buildChangePasswordCard(),
            const SizedBox(height: 20),
            _buildSessionCard(),
            const SizedBox(height: 32), // Espacio extra al final del scroll
          ],
        ),
      ),
    );
  }

  // ================= TARJETA 1: DATOS DEL PERFIL =================
  Widget _buildProfileDataCard() {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: _cardDecoration(),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Datos del perfil',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              color: Colors.black87,
            ),
          ),
          const SizedBox(height: 20),

          // Avatar Actual
          Container(
            width: 72,
            height: 72,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: const Color(0xFFFFF5F2),
              border: Border.all(color: _primaryOrange, width: 1.5),
            ),
            alignment: Alignment.center,
            child: Text(
              '?',
              style: TextStyle(
                color: _primaryOrange,
                fontSize: 28,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
          const SizedBox(height: 24),

          // Campo Nombre
          _buildFieldLabel('Nombre'),
          TextFormField(
            controller: _nameController,
            cursorColor: _primaryOrange,
            decoration: _inputDecoration(),
          ),
          const SizedBox(height: 20),

          // Selección de Avatar
          _buildFieldLabel('Foto de perfil'),
          const Text(
            'Elige una del paquete.',
            style: TextStyle(color: Colors.black54, fontSize: 12),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: List.generate(5, (index) => _buildAvatarOption(index)),
          ),
          const SizedBox(height: 32),

          // Botones Guardar/Descartar
          Row(
            children: [
              ElevatedButton(
                onPressed: () {
                  // TODO: Lógica CU-07 Guardar perfil
                },
                style: _primaryButtonStyle(),
                child: const Text('Guardar perfil'),
              ),
              const SizedBox(width: 12),
              OutlinedButton(
                onPressed: () {
                  // Reiniciar formulario
                  _nameController.text = 'Luis Vázquez';
                  setState(() => _selectedAvatarIndex = 0);
                },
                style: _secondaryButtonStyle(),
                child: const Text('Descartar'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  // ================= TARJETA 2: CAMBIAR CONTRASEÑA =================
  Widget _buildChangePasswordCard() {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: _cardDecoration(),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Cambiar contraseña',
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.bold,
              color: Colors.black87,
            ),
          ),
          const SizedBox(height: 20),

          _buildFieldLabel('Contraseña actual'),
          _buildPasswordField(
            _currentPasswordController,
            _obscureCurrentPwd,
            () {
              setState(() => _obscureCurrentPwd = !_obscureCurrentPwd);
            },
          ),
          const SizedBox(height: 16),

          _buildFieldLabel('Nueva contraseña'),
          _buildPasswordField(_newPasswordController, _obscureNewPwd, () {
            setState(() => _obscureNewPwd = !_obscureNewPwd);
          }),
          const SizedBox(height: 4),
          const Text(
            'Mínimo 8 caracteres, con al menos una letra y un número.',
            style: TextStyle(color: Colors.black54, fontSize: 12),
          ),
          const SizedBox(height: 16),

          _buildFieldLabel('Confirmar nueva contraseña'),
          _buildPasswordField(
            _confirmPasswordController,
            _obscureConfirmPwd,
            () {
              setState(() => _obscureConfirmPwd = !_obscureConfirmPwd);
            },
          ),
          const SizedBox(height: 32),

          OutlinedButton(
            onPressed: () {
              // TODO: Lógica CU-07 Cambiar contraseña
            },
            style: _secondaryButtonStyle(),
            child: const Text('Actualizar contraseña'),
          ),
        ],
      ),
    );
  }

  // ================= TARJETA 3: SESIÓN =================
  Widget _buildSessionCard() {
    return Container(
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(8),
        // En móvil, simular el borde punteado con un borde gris con ligero espaciado,
        // o utilizar un borde sólido distintivo para no requerir paquetes externos.
        border: Border.all(
          color: Colors.grey.shade400,
          style: BorderStyle.solid,
          width: 1.5,
        ),
      ),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          const Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Sesión',
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.bold,
                    color: Colors.black87,
                  ),
                ),
                SizedBox(height: 4),
                Text(
                  'Sales de Sonimbus en este equipo.',
                  style: TextStyle(color: Colors.black54, fontSize: 13),
                ),
              ],
            ),
          ),
          const SizedBox(width: 16),
          ElevatedButton(
            onPressed: _showLogoutConfirmation,
            style: _primaryButtonStyle(),
            child: const Text('Cerrar sesión'),
          ),
        ],
      ),
    );
  }

  // ================= COMPONENTES REUTILIZABLES =================

  BoxDecoration _cardDecoration() {
    return BoxDecoration(
      color: Colors.white,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: _cardBorder),
    );
  }

  Widget _buildFieldLabel(String text) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8.0),
      child: Text(
        text,
        style: const TextStyle(
          fontWeight: FontWeight.w700,
          fontSize: 13,
          color: Colors.black87,
        ),
      ),
    );
  }

  Widget _buildAvatarOption(int index) {
    final isSelected = _selectedAvatarIndex == index;
    return GestureDetector(
      onTap: () => setState(() => _selectedAvatarIndex = index),
      child: Container(
        width: 48,
        height: 48,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: Colors.grey.shade200,
          border: Border.all(
            color: isSelected ? _primaryOrange : Colors.transparent,
            width: 2.5,
          ),
        ),
        alignment: Alignment.center,
        child: Icon(
          Icons.person_outline, // Placeholder del icono del avatar
          color: isSelected ? _primaryOrange : Colors.black38,
        ),
      ),
    );
  }

  Widget _buildPasswordField(
    TextEditingController controller,
    bool isObscured,
    VoidCallback onToggle,
  ) {
    return TextFormField(
      controller: controller,
      obscureText: isObscured,
      cursorColor: _primaryOrange,
      decoration: _inputDecoration().copyWith(
        suffixIcon: IconButton(
          icon: Icon(
            isObscured
                ? Icons.visibility_off_outlined
                : Icons.visibility_outlined,
            color: Colors.black45,
            size: 20,
          ),
          onPressed: onToggle,
        ),
      ),
    );
  }

  InputDecoration _inputDecoration() {
    return InputDecoration(
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: const BorderSide(color: Colors.black26),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: _primaryOrange, width: 1.5),
      ),
    );
  }

  ButtonStyle _primaryButtonStyle() {
    return ElevatedButton.styleFrom(
      backgroundColor: _primaryOrange,
      foregroundColor: Colors.white,
      elevation: 0,
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      textStyle: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
    );
  }

  ButtonStyle _secondaryButtonStyle() {
    return OutlinedButton.styleFrom(
      foregroundColor: Colors.black87,
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
      side: const BorderSide(color: Colors.black26),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
      textStyle: const TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
    );
  }
}
