angzarr-codegen-descriptors = the descriptor set does not decode: { $detail }
angzarr-codegen-file-not-in-set = { $file } is not in the descriptor set
angzarr-codegen-unknown-component = { $message } names the component { $component }, which no message declares
angzarr-codegen-unknown-emit = { $command } emits { $event }, which is not an event of its component
angzarr-codegen-unknown-refusal = { $command } refuses { $refusal }, which is not a message of the descriptor set
angzarr-codegen-unowned-domain = { $policy } names the domain { $domain }, which no aggregate or process manager owns
angzarr-codegen-unsupported-kind = { $component } is a component of kind { $kind }, which this generator does not emit
angzarr-codegen-commutative = { $component }'s state lists COMMUTATIVE, which is not generated yet
angzarr-codegen-oneof = { $message } has the oneof { $oneof }, which is not mapped yet
angzarr-codegen-nested-type = { $message } declares { $nested } inside it; declare it at the top level
angzarr-codegen-well-known-type = { $message }.{ $field } is a { $type_name }; only google.protobuf.Timestamp is mapped
angzarr-codegen-enum-zero = { $enum_name }'s zero value is { $value }; name it *_UNSPECIFIED
angzarr-codegen-no-wire-path = no Rust path is given for the prost types of package { $package }
angzarr-codegen-translate-unknown-policy = translate names the policy { $policy }, which no message declares
angzarr-codegen-translate-not-a-target-command = translate names { $command }, which is not a command of { $policy }'s target
angzarr-codegen-translate-not-a-source-fact = translate names { $fact }, which is not an input of { $policy }'s source
angzarr-codegen-scaffold-impl-not-found = { $path } holds no decorated impl of { $component }; it is left as it is
angzarr-codegen-refuses-on-fact = { $message } is a fact and declares refuses; a fact cannot be refused
angzarr-codegen-reserved-name = { $message } is named { $name }, which the generated domain layer gives the runtime's own { $name }; rename it
angzarr-codegen-translate-not-a-fact = translate names { $fact }, a command of { $policy }'s source; it must name a fact
angzarr-codegen-several-refusals = { $command } declares more than one refusal, which the scaffold does not emit yet
angzarr-codegen-input-mismatch = { $path } takes { $message } as { $found }, but the proto declares it a { $expected }
angzarr-codegen-python-no-domain = the protos declare no aggregate of the domain { $domain }
angzarr-codegen-python-kind = { $component } is not an aggregate; the Python scaffold emits aggregates only
angzarr-codegen-python-fact = { $message } is a fact, which the Python scaffold does not emit yet
angzarr-codegen-python-refuses = { $message } declares refuses, which the Python scaffold does not emit yet
angzarr-codegen-startup-packages = { $files } declare components of more than one package, which the startup does not lay out yet
angzarr-codegen-startup-config = the host config does not read: { $detail }

## Startup lints.

angzarr-codegen-startup-unknown-language = the domain { $domain } is in { $language }, which has no default launch; give it a launch
angzarr-codegen-startup-empty-launch = the domain { $domain }'s launch names no program
angzarr-codegen-startup-unknown-domain = the domain { $domain } is owned by no aggregate or process manager of the protos
angzarr-codegen-startup-duplicate-domain = the domain { $domain } is listed more than once
angzarr-codegen-startup-unlisted-domain = the protos declare the domain { $domain }, which the config does not list
