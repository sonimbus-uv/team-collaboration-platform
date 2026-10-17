# -*- mode: ruby -*-
# vi: set ft=ruby :

# Proveedores soportados (ADR-006):
#   vagrant up --provider=virtualbox   (Windows, macOS y Linux)
#   vagrant up --provider=libvirt      (Linux con KVM; requiere vagrant-libvirt y virtiofsd)

VM_MEMORY = ENV.fetch("VM_MEMORY", "6144").to_i
VM_CPUS = ENV.fetch("VM_CPUS", "4").to_i

Vagrant.configure("2") do |config|

  config.vm.box = "bento/ubuntu-24.04"
  config.vm.hostname = "Nimbus-Box"

  config.vm.boot_timeout = 600

  config.vm.network "private_network",
    ip: "192.168.33.30"

  config.vm.provider "virtualbox" do |vb, override|
    override.vm.disk :disk,
      size: "80GB",
      primary: true

    vb.name = "team-collaboration-platform"
    vb.memory = VM_MEMORY
    vb.cpus = VM_CPUS
    vb.gui = false
  end

  # bento/ubuntu-24.04 no se publica para libvirt; se usa la box oficial de Canonical.
  config.vm.provider "libvirt" do |lv, override|
    override.vm.box = "cloud-image/ubuntu-24.04"

    # libvirt no tiene las carpetas compartidas de VirtualBox; virtiofs es bidireccional.
    override.vm.synced_folder ".", "/vagrant", type: "virtiofs"
    lv.memorybacking :access, mode: "shared"

    lv.default_prefix = "team-collaboration-platform"
    lv.driver = "kvm"
    lv.memory = VM_MEMORY
    lv.cpus = VM_CPUS
    lv.machine_virtual_size = 80
  end

  config.vm.provision "shell",
    path: "scripts/provision.sh",
    privileged: true

end
