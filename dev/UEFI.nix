# NOTICE: All configuration in this file is just for testing
# Not part of vaultix deploy requirements
{ ... }:
{
  boot = {
    kernelParams = [
      "audit=0"
      "net.ifnames=0"

      "console=ttyS0"
      "earlyprintk=ttyS0"
      "rootdelay=300"
    ];
    loader = {
      efi = {
        canTouchEfiVariables = true;
        efiSysMountPoint = "/efi";
      };
      systemd-boot.enable = true;
      timeout = 3;
    };
    initrd = {
      compressor = "zstd";
      compressorArgs = [
        "-19"
        "-T0"
      ];
      systemd.enable = true;

      kernelModules = [
        "hv_vmbus" # for hyper-V
        "hv_netvsc"
        "hv_utils"
        "hv_storvsc"
      ];
    };
  };
  disko = {
    enableConfig = true;

    devices = {
      disk.main = {
        device = "/dev/sda";
        type = "disk";
        content = {
          type = "gpt";
          partitions = {
            ESP = {
              name = "ESP";
              size = "512M";
              type = "EF00";
              priority = 0;
              content = {
                type = "filesystem";
                format = "vfat";
                mountpoint = "/efi";
                mountOptions = [
                  "fmask=0077"
                  "dmask=0077"
                ];
              };
            };

            root = {
              label = "ROOT";
              end = "-0";
              content = {
                type = "btrfs";
                extraArgs = [
                  "-f"
                  "--csum xxhash64"
                ];
                subvolumes = {
                  "boot" = {
                    mountpoint = "/boot";
                    mountOptions = [
                      "compress=zstd"
                      "noatime"
                    ];
                  };
                  "root" = {
                    mountpoint = "/";
                    mountOptions = [
                      "compress=zstd"
                      "noatime"
                      "nodev"
                      "nosuid"
                    ];
                  };
                };
              };
            };
          };
        };
      };
    };
  };
}
