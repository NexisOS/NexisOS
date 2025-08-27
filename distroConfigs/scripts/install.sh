#!/bin/sh

# Set whiptail dialog colors
export NEWT_COLORS='
root=,grey
window=,black
shadow=,blue
border=blue,black
title=blue,black
textbox=blue,black
radiolist=black,black
label=black,blue
checkbox=black,blue
compactbutton=black,blue
button=black,red
button.focus=white,red
menu=black,blue
menu.focus=white,red
'

set -e

# ========== Safety & Environment Checks ==========

# Require root
[ "$(id -u)" -eq 0 ] || {
  echo "Must run as root" >&2
  exit 1
}

# Required tools check
for tool in whiptail parted mkfs.ext4 tar grub-install chroot; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "Missing required tool: $tool" >&2
    exit 1
  }
done

# Required files check
REQUIRED_FILES="/nexis/rootfs.tar.gz /nexis/init/dinit.tar.gz"
for f in $REQUIRED_FILES; do
  [ -f "$f" ] || {
    echo "Missing required file: $f" >&2
    exit 1
  }
done

# ========== UI Prompts ==========

whiptail --msgbox "Welcome to NexisOS Installer" 6 40

DISK=$(whiptail --title "Disk Selection" --menu "Select disk" 15 50 4 \
  /dev/sda "Primary Disk" \
  /dev/sdb "Secondary Disk" \
  /dev/nvme0n1 "NVMe SSD" 3>&1 1>&2 2>&3)

KERNEL_TYPE=$(whiptail --title "Kernel" --menu "Select kernel" 10 40 2 \
  vanilla "Vanilla Kernel" \
  hardened "Hardened Kernel" 3>&1 1>&2 2>&3)

DE_TYPE=$(whiptail --title "Desktop Env" --menu "Choose DE / WM" 15 50 5 \
  i3 "i3 WM" \
  xfce "XFCE DE" \
  gnome "GNOME DE" \
  none "No GUI" 3>&1 1>&2 2>&3)

if ! whiptail --yesno "Install NexisOS on $DISK with:\n\nKernel: $KERNEL_TYPE\nDE: $DE_TYPE\n\nContinue?" 12 60; then
  whiptail --msgbox "Installation cancelled." 6 40
  exit 0
fi

if ! whiptail --yesno "⚠️  All data on $DISK will be ERASED.\nAre you sure you want to continue?" 10 60; then
  exit 0
fi

# ========== Install Process ==========

MNT="/mnt/nexisos"
STEP=0

# Partition suffix
case "$DISK" in
  *nvme*) PART="${DISK}p1" ;;
  *)      PART="${DISK}1" ;;
esac

# Ensure not already mounted
mountpoint -q "$MNT" && {
  echo "$MNT is already mounted. Unmount first." >&2
  exit 1
}

# Progress helper
progress() {
  echo "XXX"
  echo "$1"
  echo "XXX"
  echo "$2"
}

# Run install steps with progress bar
(
progress "Partitioning disk..." $((STEP+=10))
parted -s "$DISK" mklabel gpt
parted -s "$DISK" mkpart primary ext4 1MiB 100%
sleep 1

progress "Formatting partition..." $((STEP+=10))
mkfs.ext4 "$PART"

progress "Mounting partition..." $((STEP+=5))
mkdir -p "$MNT"
mount "$PART" "$MNT"

progress "Extracting base system..." $((STEP+=25))
tar -xzf /nexis/rootfs.tar.gz -C "$MNT"

progress "Installing $KERNEL_TYPE kernel..." $((STEP+=10))
cp "/nexis/kernels/${KERNEL_TYPE}-vmlinuz" "$MNT/boot/vmlinuz"

progress "Installing init system..." $((STEP+=10))
tar -xzf /nexis/init/dinit.tar.gz -C "$MNT"
ln -sf /dinit/dinit "$MNT/init"

progress "Copying initrd image..." $((STEP+=5))
[ -f /nexis/initrd.img ] && cp /nexis/initrd.img "$MNT/boot/initrd.img"

progress "Installing Desktop Environment..." $((STEP+=15))
chroot "$MNT" /bin/sh -c "
  nexpkg update
  case $DE_TYPE in
    i3) nexpkg install i3 i3status i3lock ;;
    xfce) nexpkg install xfce lightdm ;;
    gnome) nexpkg install gnome gdm ;;
    none) echo 'No GUI selected.' ;;
  esac
"

progress "Installing GRUB bootloader..." $((STEP+=5))
grub-install --target=i386-pc --boot-directory="$MNT/boot" "$DISK"

cat > "$MNT/boot/grub/grub.cfg" <<EOF
set default=0
set timeout=5

menuentry "NexisOS" {
    linux /boot/vmlinuz
    initrd /boot/initrd.img
}
EOF

progress "Finishing installation..." 100

) | whiptail --title "Installing NexisOS..." --gauge "Please wait..." 10 60 0

whiptail --msgbox "Installation complete! Rebooting now." 6 40
reboot
