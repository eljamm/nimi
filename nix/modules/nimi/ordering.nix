{ lib, config, ... }:
let
  inherit (lib)
    mkOption
    types
    ;

  serviceNames = lib.attrNames config.services;
  orderingKeys = lib.attrNames config.ordering;
in
{
  _class = "nimi";

  options.ordering = mkOption {
    description = ''
      Service startup ordering constraints.

      Each attribute names a service and declares which other services
      it must wait for before starting. Services without ordering
      constraints (or not mentioned here) start immediately.

      This only controls startup order inside a single nimi instance.
      It applies equally to containers, NixOS, Home Manager, and
      local development runs.
    '';
    example = lib.literalExpression ''
      {
        backend.after  = [ "database" ];
        frontend.after = [ "database" "backend" ];
        api.before = [ "backend" ];
        web.wants = [ "api" ];
        critical.requires = [ "database" ];
      }
    '';
    type = types.attrsOf (
      types.submodule {
        options.after = mkOption {
          description = ''
            List of service names that must have started before this
            service is spawned.
          '';
          type = types.listOf types.str;
          default = [ ];
        };
        options.afterReady = mkOption {
          description = ''
            List of service names that must be ready before this
            service is spawned. Each target must declare a
            readiness check.
          '';
          type = types.listOf types.str;
          default = [ ];
        };
        options.before = mkOption {
          description = ''
            List of service names that should start before this service.
            This is a soft ordering - the service will start after these,
            but doesn't fail if they don't exist.
          '';
          type = types.listOf types.str;
          default = [ ];
        };
        options.wants = mkOption {
          description = ''
            List of service names this service wants (soft dependency).
            The service will start after these, but won't fail if they fail.
          '';
          type = types.listOf types.str;
          default = [ ];
        };
        options.requires = mkOption {
          description = ''
            List of service names this service requires (hard dependency).
            The service will fail to start if these don't start successfully.
          '';
          type = types.listOf types.str;
          default = [ ];
        };
        options.wantedBy = mkOption {
          description = ''
            List of service names that want this service.
            (reverse of 'wants')
          '';
          type = types.listOf types.str;
          default = [ ];
        };
        options.requiredBy = mkOption {
          description = ''
            List of service names that require this service.
            (reverse of 'requires')
          '';
          type = types.listOf types.str;
          default = [ ];
        };
      }
    );
    default = { };
  };

  config.assertions =
    let
      mkKeyAssertion = name: {
        assertion = lib.elem name serviceNames;
        message = "ordering.${name} references a service that does not exist.";
      };

      mkAfterAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.after references unknown service \"${dep}\".";
        }) deps;

      mkAfterReadyAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.afterReady references unknown service \"${dep}\".";
        }) deps
        ++ map (dep: {
          assertion = config.services.${dep}.readyCheck != null;
          message = "ordering.${name}.afterReady references service \"${dep}\" without readyCheck.";
        }) deps;

      mkBeforeAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.before references unknown service \"${dep}\".";
        }) deps;

      mkWantsAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.wants references unknown service \"${dep}\".";
        }) deps;

      mkRequiresAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.requires references unknown service \"${dep}\".";
        }) deps;

      mkWantedByAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.wantedBy references unknown service \"${dep}\".";
        }) deps;

      mkRequiredByAssertions =
        name: deps:
        map (dep: {
          assertion = lib.elem dep serviceNames;
          message = "ordering.${name}.requiredBy references unknown service \"${dep}\".";
        }) deps;
    in
    (map mkKeyAssertion orderingKeys)
    ++ (lib.concatLists (lib.mapAttrsToList (name: o: mkAfterAssertions name o.after) config.ordering))
    ++ (lib.concatLists (
      lib.mapAttrsToList (name: o: mkAfterReadyAssertions name o.afterReady) config.ordering
    ))
    ++ (lib.concatLists (
      lib.mapAttrsToList (name: o: mkBeforeAssertions name (o.before or [ ])) config.ordering
    ))
    ++ (lib.concatLists (
      lib.mapAttrsToList (name: o: mkWantsAssertions name (o.wants or [ ])) config.ordering
    ))
    ++ (lib.concatLists (
      lib.mapAttrsToList (name: o: mkRequiresAssertions name (o.requires or [ ])) config.ordering
    ))
    ++ (lib.concatLists (
      lib.mapAttrsToList (name: o: mkWantedByAssertions name (o.wantedBy or [ ])) config.ordering
    ))
    ++ (lib.concatLists (
      lib.mapAttrsToList (name: o: mkRequiredByAssertions name (o.requiredBy or [ ])) config.ordering
    ));
}
