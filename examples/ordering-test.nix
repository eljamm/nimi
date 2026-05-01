{
  nimi,
  pkgs,
  lib,
}:

let
  scriptBin = name: text: pkgs.writeShellScriptBin name text;
in

nimi.mkNimiBin {
  settings.binName = "ordering-test";

  services.python-web = {
    process.argv = [ (lib.getExe (scriptBin "web" ''echo "web starting"; sleep 10'')) ];
  };

  services.python-web-hello = {
    process.argv = [ (lib.getExe (scriptBin "hello" ''echo "hello world"'')) ];
  };

  services.setup-once = {
    process.argv = [ (lib.getExe (scriptBin "setup-once" ''echo "setup-once running (oneshot)"'')) ];
    type = "oneshot";
  };

  ordering.python-web-hello.after = [ "python-web" ];
  ordering.python-web-hello.requires = [ "python-web" ];
  ordering.python-web.after = [ "setup-once" ];
}

