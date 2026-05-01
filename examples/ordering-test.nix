{
  nimi,
  pkgs,
  lib,
}:
nimi.mkNimiBin {
  settings.binName = "ordering-test";

  components = {
    python-web = {
      command = pkgs.writeShellScriptBin "web" ''
        echo "web starting"
        sleep 10
      '';
    };

    python-web-hello = {
      command = pkgs.writeShellScriptBin "hello" ''
        echo "hello world"
      '';
    };
  };

  ordering.python-web-hello.after = [ "python-web" ];
  ordering.python-web-hello.requires = [ "python-web" ];
}

