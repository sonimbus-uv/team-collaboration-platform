import 'package:flutter/material.dart';

class RegisterScreen extends StatefulWidget {
  const RegisterScreen({super.key});

  @override
  State<RegisterScreen> createState() => _RegisterScreenState();
}

class _RegisterScreenState extends State<RegisterScreen> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();

  bool _obscurePassword = true;
  bool _obscureConfirmPassword = true;

  // Estados de error basados en el FXML
  bool _hasGeneralError = false;
  String _generalErrorMessage = '';
  bool _showResendLink = false;
  bool _hasConfirmPasswordError = false;

  final Color _primaryOrange = const Color(0xFFC44111);

  @override
  void dispose() {
    _nameController.dispose();
    _emailController.dispose();
    _passwordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  void _togglePasswordVisibility() {
    setState(() => _obscurePassword = !_obscurePassword);
  }

  void _toggleConfirmPasswordVisibility() {
    setState(() => _obscureConfirmPassword = !_obscureConfirmPassword);
  }

  void _handleRegister() {
    if (_passwordController.text != _confirmPasswordController.text) {
      setState(() => _hasConfirmPasswordError = true);
      return;
    } else {
      setState(() => _hasConfirmPasswordError = false);
    }

    // TODO: Conectar con Auth Service (RF-AUTH-01)
    if (_nameController.text.isEmpty ||
        _emailController.text.isEmpty ||
        _passwordController.text.isEmpty) {
      setState(() {
        _hasGeneralError = true;
        _generalErrorMessage = 'Completa todos los campos correctamente.';
        _showResendLink = false;
      });
    } else {
      setState(() => _hasGeneralError = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFFF7F7F9),
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24.0),
            child: Container(
              constraints: const BoxConstraints(maxWidth: 420),
              padding: const EdgeInsets.symmetric(
                horizontal: 32.0,
                vertical: 40.0,
              ),
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(12),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withOpacity(0.05),
                    blurRadius: 15,
                    offset: const Offset(0, 5),
                  ),
                ],
              ),
              child: Form(
                key: _formKey,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // Logo
                    Align(
                      alignment: Alignment.center,
                      child: Container(
                        width: 64,
                        height: 64,
                        decoration: BoxDecoration(
                          color: _primaryOrange,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        alignment: Alignment.center,
                        child: const Text(
                          '[Logo]',
                          style: TextStyle(
                            color: Colors.white,
                            fontWeight: FontWeight.bold,
                            fontSize: 16,
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 24),

                    const Text(
                      'Crear cuenta',
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        fontSize: 26,
                        fontWeight: FontWeight.w800,
                        color: Colors.black87,
                      ),
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      'Todos los campos son obligatorios',
                      textAlign: TextAlign.center,
                      style: TextStyle(fontSize: 15, color: Colors.black54),
                    ),
                    const SizedBox(height: 32),

                    if (_hasGeneralError)
                      Container(
                        padding: const EdgeInsets.all(12),
                        margin: const EdgeInsets.only(bottom: 24.0),
                        decoration: BoxDecoration(
                          color: Colors.red.shade50,
                          border: Border.all(color: Colors.red.shade200),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Icon(
                              Icons.error_outline,
                              color: Colors.red.shade700,
                              size: 20,
                            ),
                            const SizedBox(width: 10),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    _generalErrorMessage,
                                    style: TextStyle(
                                      color: Colors.red.shade900,
                                      fontSize: 14,
                                    ),
                                  ),
                                  if (_showResendLink)
                                    Padding(
                                      padding: const EdgeInsets.only(top: 4.0),
                                      child: GestureDetector(
                                        onTap: () {},
                                        child: Text(
                                          '¿No recibiste el correo de verificación?',
                                          style: TextStyle(
                                            color: _primaryOrange,
                                            decoration:
                                                TextDecoration.underline,
                                            fontSize: 13,
                                          ),
                                        ),
                                      ),
                                    ),
                                ],
                              ),
                            ),
                          ],
                        ),
                      ),

                    const Text(
                      'Nombre completo',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        fontSize: 13,
                        color: Colors.black87,
                      ),
                    ),
                    const SizedBox(height: 8),
                    TextFormField(
                      controller: _nameController,
                      keyboardType: TextInputType.name,
                      cursorColor: _primaryOrange,
                      decoration: _buildInputDecoration('Tu nombre'),
                    ),
                    const SizedBox(height: 20),

                    const Text(
                      'Correo electrónico',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        fontSize: 13,
                        color: Colors.black87,
                      ),
                    ),
                    const SizedBox(height: 8),
                    TextFormField(
                      controller: _emailController,
                      keyboardType: TextInputType.emailAddress,
                      cursorColor: _primaryOrange,
                      decoration: _buildInputDecoration('nombre@correo.com'),
                    ),
                    const SizedBox(height: 20),

                    const Text(
                      'Contraseña',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        fontSize: 13,
                        color: Colors.black87,
                      ),
                    ),
                    const SizedBox(height: 8),
                    TextFormField(
                      controller: _passwordController,
                      obscureText: _obscurePassword,
                      cursorColor: _primaryOrange,
                      decoration: _buildInputDecoration('••••••••').copyWith(
                        suffixIcon: _buildVisibilityToggle(
                          _obscurePassword,
                          _togglePasswordVisibility,
                        ),
                      ),
                    ),
                    const SizedBox(height: 6),
                    const Text(
                      'De 8 a 128 caracteres, con al menos una mayúscula, una minúscula, un número y un símbolo.',
                      style: TextStyle(
                        color: Colors.black54,
                        fontSize: 12,
                        height: 1.3,
                      ),
                    ),
                    const SizedBox(height: 20),

                    const Text(
                      'Confirmar contraseña',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        fontSize: 13,
                        color: Colors.black87,
                      ),
                    ),
                    const SizedBox(height: 8),
                    TextFormField(
                      controller: _confirmPasswordController,
                      obscureText: _obscureConfirmPassword,
                      cursorColor: _primaryOrange,
                      decoration: _buildInputDecoration('••••••••').copyWith(
                        suffixIcon: _buildVisibilityToggle(
                          _obscureConfirmPassword,
                          _toggleConfirmPasswordVisibility,
                        ),
                      ),
                    ),

                    if (_hasConfirmPasswordError)
                      Padding(
                        padding: const EdgeInsets.only(top: 6.0),
                        child: Row(
                          children: [
                            Icon(
                              Icons.error,
                              color: Colors.red.shade700,
                              size: 14,
                            ),
                            const SizedBox(width: 6),
                            Text(
                              'Las contraseñas no coinciden.',
                              style: TextStyle(
                                color: Colors.red.shade900,
                                fontSize: 12,
                              ),
                            ),
                          ],
                        ),
                      ),
                    const SizedBox(height: 32),

                    ElevatedButton(
                      style: ElevatedButton.styleFrom(
                        backgroundColor: _primaryOrange,
                        foregroundColor: Colors.white,
                        padding: const EdgeInsets.symmetric(vertical: 16),
                        elevation: 0,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(6),
                        ),
                      ),
                      onPressed: _handleRegister,
                      child: const Text(
                        'Crear cuenta',
                        style: TextStyle(
                          fontSize: 15,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                    const SizedBox(height: 24),

                    Row(
                      children: [
                        const Expanded(child: Divider(color: Colors.black12)),
                        Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 16),
                          child: Text(
                            'o',
                            style: TextStyle(
                              color: Colors.grey.shade600,
                              fontSize: 13,
                            ),
                          ),
                        ),
                        const Expanded(child: Divider(color: Colors.black12)),
                      ],
                    ),
                    const SizedBox(height: 24),

                    OutlinedButton.icon(
                      style: OutlinedButton.styleFrom(
                        padding: const EdgeInsets.symmetric(vertical: 14),
                        side: const BorderSide(color: Colors.black26),
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(6),
                        ),
                        foregroundColor: Colors.black87,
                      ),
                      onPressed: () {},
                      icon: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 2,
                        ),
                        decoration: BoxDecoration(
                          border: Border.all(color: Colors.black26),
                          borderRadius: BorderRadius.circular(4),
                          color: const Color(0xFFFFF5F2),
                        ),
                        child: Text(
                          'G',
                          style: TextStyle(
                            fontWeight: FontWeight.bold,
                            fontSize: 13,
                            color: _primaryOrange,
                          ),
                        ),
                      ),
                      label: const Text(
                        'Continuar con Google',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ),
                    const SizedBox(height: 32),

                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        const Text(
                          '¿Ya tienes cuenta? ',
                          style: TextStyle(color: Colors.black54, fontSize: 14),
                        ),
                        GestureDetector(
                          onTap: () {
                            Navigator.pop(context); // Regresa al Login
                          },
                          child: Text(
                            'Iniciar sesión',
                            style: TextStyle(
                              color: _primaryOrange,
                              fontWeight: FontWeight.w800,
                              decoration: TextDecoration.underline,
                              decorationColor: _primaryOrange,
                              fontSize: 14,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  InputDecoration _buildInputDecoration(String hint) {
    return InputDecoration(
      hintText: hint,
      hintStyle: const TextStyle(color: Colors.black38),
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(6)),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: _primaryOrange, width: 2),
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
    );
  }

  Widget _buildVisibilityToggle(bool isObscured, VoidCallback onPressed) {
    return Container(
      margin: const EdgeInsets.only(right: 8, top: 4, bottom: 4),
      decoration: const BoxDecoration(
        border: Border(left: BorderSide(color: Colors.black12)),
      ),
      child: TextButton(
        onPressed: onPressed,
        style: TextButton.styleFrom(
          foregroundColor: Colors.black87,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(4)),
        ),
        child: Text(
          isObscured ? 'Mostrar' : 'Ocultar',
          style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13),
        ),
      ),
    );
  }
}
