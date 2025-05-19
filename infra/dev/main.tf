locals {
  vms = {
    "us-vm" = {
      location = "eastus"
      name     = "${var.prefix}-us"
    }
    "asia-vm" = {
      location = "southeastasia"
      name     = "${var.prefix}-asia"
    }
  }
}

resource "azurerm_resource_group" "main" {
  for_each = local.vms
  name     = "${each.value.name}-rg"
  location = each.value.location
}

resource "azurerm_virtual_network" "main" {
  for_each            = local.vms
  name                = "${each.value.name}-vnet"
  address_space       = ["10.0.0.0/22"]
  location            = each.value.location
  resource_group_name = azurerm_resource_group.main[each.key].name
}

resource "azurerm_subnet" "internal" {
  for_each             = local.vms
  name                 = "internal"
  resource_group_name  = azurerm_resource_group.main[each.key].name
  virtual_network_name = azurerm_virtual_network.main[each.key].name
  address_prefixes     = ["10.0.2.0/24"]
}

resource "azurerm_network_interface" "main" {
  for_each            = local.vms
  name                = "${each.value.name}-nic"
  resource_group_name = azurerm_resource_group.main[each.key].name
  location            = each.value.location

  ip_configuration {
    name                          = "internal"
    subnet_id                     = azurerm_subnet.internal[each.key].id
    private_ip_address_allocation = "Dynamic"
  }
}

resource "azurerm_linux_virtual_machine" "main" {
  for_each                        = local.vms
  name                            = "${each.value.name}-vm"
  resource_group_name             = azurerm_resource_group.main[each.key].name
  location                        = each.value.location
  size                            = "Standard_B2s"
  admin_username                  = "adminuser"
  admin_password                  = "@Windexer"
  disable_password_authentication = false

  network_interface_ids = [
    azurerm_network_interface.main[each.key].id,
  ]

  source_image_reference {
    publisher = "Canonical"
    offer     = "UbuntuServer"
    sku       = "18.04-LTS"
    version   = "latest"
  }

  os_disk {
    storage_account_type = "Standard_LRS"
    caching              = "ReadWrite"
  }
}