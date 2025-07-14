locals {
  vm_config_map       = var.vm_config_map
  vm_base_config_map  = var.vm_base_config_map
  disk_configurations = var.disk_configurations
}

resource "proxmox_qemu_vm" "this" {
  for_each = {
    for instance in var.instances : instance.vmname => instance
  }

  # Core VM Identity
  vmid        = each.value.vmid
  name        = each.key
  target_node = var.target_node
  tags        = var.tags

  # Clone/Template Settings (used when cloning a vm from a template)
  clone      = var.template_name
  full_clone = false
  os_type    = var.os_type

  # Hardware Configuration
  bios     = local.vm_config_map.bios
  machine  = local.vm_config_map.machine
  cpu_type = lookup(local.vm_base_config_map, "cpu", "host")
  cores    = local.vm_config_map.cores
  sockets  = lookup(local.vm_base_config_map, "sockets", 1)
  vcpus    = lookup(local.vm_base_config_map, "vcpus", null)
  memory   = local.vm_config_map.memory
  balloon  = local.vm_config_map.balloon

  # Boot Configuration
  boot     = local.vm_config_map.boot
  bootdisk = local.vm_config_map.bootdisk
  onboot   = local.vm_config_map.onboot
  start    = try(local.vm_config_map.start, true)

  scsihw = local.vm_config_map.scsihw
  agent   = 1
  qemu_os = lookup(local.vm_base_config_map, "qemu_os", "l26")

  # Cloud-Init Configuration
  ipconfig0  = each.value.ipconfig
  ciuser     = try(local.vm_config_map.ciuser, null)
  cipassword = var.cipassword
  ciupgrade  = try(local.vm_config_map.ciupgrade, false)
  sshkeys    = var.sshkeys

  # Network Settings
  skip_ipv4 = lookup(local.vm_base_config_map, "skip_ipv4", null)
  skip_ipv6 = lookup(local.vm_base_config_map, "skip_ipv6", null)

  # Timing & Behavior Settings
  additional_wait        = lookup(local.vm_base_config_map, "additional_wait", 15)
  automatic_reboot       = lookup(local.vm_base_config_map, "automatic_reboot", true)
  clone_wait             = lookup(local.vm_base_config_map, "clone_wait", 30)
  define_connection_info = local.vm_config_map.define_connection_info

  # Network Interface
  network {
    id        = 0
    bridge    = lookup(var.network, "bridge", "vmbr0")
    model     = lookup(var.network, "model", "virtio")
    macaddr   = lookup(each.value, "macaddr", null)
    firewall  = lookup(var.network, "firewall", false)
    link_down = lookup(var.network, "link_down", false)
    tag       = lookup(var.network, "tag", null)
    mtu       = lookup(var.network, "mtu", null)
    queues    = lookup(var.network, "queues", null)
    rate      = lookup(var.network, "rate", null)
  }

  # Disk Configuration
  dynamic "disk" {
    for_each = try(local.disk_configurations.scsi, {})
    content {
      slot    = disk.key
      type    = "scsi"
      storage = disk.value.disk.storage
      size    = disk.value.disk.size
      format  = try(disk.value.disk.format, "raw")

      # Performance Settings
      discard    = try(disk.value.disk.discard, false)
      emulatessd = try(disk.value.disk.emulatessd, false)
      iothread   = try(disk.value.disk.iothread, false)

      # Data Protection
      backup    = try(disk.value.disk.backup, true)
      replicate = try(disk.value.disk.replicate, false)
      readonly  = try(disk.value.disk.readonly, false)

      # IO Limits
      iops_r_burst         = try(disk.value.disk.iops_r_burst, null)
      iops_r_burst_length  = try(disk.value.disk.iops_r_burst_length, null)
      iops_r_concurrent    = try(disk.value.disk.iops_r_concurrent, null)
      iops_wr_burst        = try(disk.value.disk.iops_wr_burst, null)
      iops_wr_burst_length = try(disk.value.disk.iops_wr_burst_length, null)
      iops_wr_concurrent   = try(disk.value.disk.iops_wr_concurrent, null)

      # Bandwidth Limits
      mbps_r_burst      = try(disk.value.disk.mbps_r_burst, null)
      mbps_r_concurrent = try(disk.value.disk.mbps_r_concurrent, null)
      mbps_wr_burst     = try(disk.value.disk.mbps_wr_burst, null)
      mbps_wr_concurrent = try(disk.value.disk.mbps_wr_concurrent, null)
    }
  }

  dynamic "cdrom" {
    for_each = try(local.disk_configurations.ide.ide2, null) != null ? [1] : []
    content {
      slot = "ide2"
      iso  = local.disk_configurations.ide.ide2.cdrom.iso
    }
  }

  dynamic "cloudinit_drive" {
    for_each = try(local.disk_configurations.ide.ide3, null) != null ? [1] : []
    content {
      slot    = "ide3"
      storage = local.disk_configurations.ide.ide3.cloudinit.storage
    }
  }

  dynamic "serial" {
    for_each = var.serial != null ? [var.serial] : []
    content {
      id   = serial.value.id
      type = serial.value.type
    }
  }

  dynamic "efidisk" {
    for_each = var.efidisk != null ? [var.efidisk] : []
    content {
      efitype = efidisk.value.efitype
      storage = efidisk.value.storage
    }
  }
}