import type { CommandOption, CommandChoice } from "../types/index.js";

// ============================================================================
// SlashCommandOptionBuilder
// ============================================================================

/** Option type values that match the server schema. */
export const OptionType = {
  SubCommand: 1,
  SubCommandGroup: 2,
  String: 3,
  Integer: 4,
  Boolean: 5,
  User: 6,
  Channel: 7,
  Role: 8,
  Mentionable: 9,
  Number: 10,
  Attachment: 11,
} as const;

export type OptionType = (typeof OptionType)[keyof typeof OptionType];

export class SlashCommandOptionBuilder {
  private data: Partial<CommandOption> & Pick<CommandOption, "name" | "description" | "option_type"> = {
    name: "",
    description: "",
    option_type: OptionType.String,
    required: false,
    choices: [],
    options: [],
    autocomplete: false,
    min_value: null,
    max_value: null,
  };

  setType(type: OptionType): this {
    this.data.option_type = type;
    return this;
  }

  setName(name: string): this {
    this.data.name = name;
    return this;
  }

  setDescription(description: string): this {
    this.data.description = description;
    return this;
  }

  setRequired(required = true): this {
    this.data.required = required;
    return this;
  }

  addChoice(name: string, value: string | number): this {
    this.data.choices ??= [];
    this.data.choices.push({ name, value });
    return this;
  }

  addChoices(...choices: CommandChoice[]): this {
    this.data.choices ??= [];
    this.data.choices.push(...choices);
    return this;
  }

  setMinValue(min: number): this {
    this.data.min_value = min;
    return this;
  }

  setMaxValue(max: number): this {
    this.data.max_value = max;
    return this;
  }

  setAutocomplete(autocomplete = true): this {
    this.data.autocomplete = autocomplete;
    return this;
  }

  addOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this {
    const opt = fn(new SlashCommandOptionBuilder());
    this.data.options ??= [];
    this.data.options.push(opt.build());
    return this;
  }

  build(): CommandOption {
    if (!this.data.name) throw new Error("Option name is required");
    if (!this.data.description) throw new Error("Option description is required");
    return this.data as CommandOption;
  }
}

// ============================================================================
// SlashCommandBuilder
// ============================================================================

/** Fluent builder for creating slash commands. */
export class SlashCommandBuilder {
  private name = "";
  private description = "";
  private options: CommandOption[] = [];
  private command_type = 1; // ChatInput
  private default_member_permissions: string | undefined;
  private dm_permission = true;

  /** Set the command name (1-32 chars, lowercase, no spaces). */
  setName(name: string): this {
    this.name = name.toLowerCase().replace(/\s+/g, "-");
    return this;
  }

  /** Set the command description (1-100 chars). */
  setDescription(description: string): this {
    this.description = description;
    return this;
  }

  /** Set the command type (1=ChatInput, 2=User, 3=Message). */
  setType(type: 1 | 2 | 3): this {
    this.command_type = type;
    return this;
  }

  /** Restrict to users with these permissions (bitfield string). */
  setDefaultMemberPermissions(permissions: string | number): this {
    this.default_member_permissions = String(permissions);
    return this;
  }

  /** Whether the command is available in DMs (default true). */
  setDMPermission(dmPermission: boolean): this {
    this.dm_permission = dmPermission;
    return this;
  }

  /** Add a string option. */
  addStringOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.String));
    this.options.push(opt.build());
    return this;
  }

  /** Add an integer option. */
  addIntegerOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.Integer));
    this.options.push(opt.build());
    return this;
  }

  /** Add a boolean option. */
  addBooleanOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.Boolean));
    this.options.push(opt.build());
    return this;
  }

  /** Add a user option. */
  addUserOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.User));
    this.options.push(opt.build());
    return this;
  }

  /** Add a channel option. */
  addChannelOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.Channel));
    this.options.push(opt.build());
    return this;
  }

  /** Add a role option. */
  addRoleOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.Role));
    this.options.push(opt.build());
    return this;
  }

  /** Add a number (float) option. */
  addNumberOption(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.Number));
    this.options.push(opt.build());
    return this;
  }

  /** Add a sub-command. */
  addSubCommand(
    fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder
  ): this {
    const opt = fn(new SlashCommandOptionBuilder().setType(OptionType.SubCommand));
    this.options.push(opt.build());
    return this;
  }

  /** Produce the plain object to send to the API. */
  build(): {
    name: string;
    description: string;
    options: CommandOption[];
    command_type: number;
    default_member_permissions?: string;
    dm_permission: boolean;
  } {
    if (!this.name) throw new Error("Command name is required");
    if (!this.description && this.command_type === 1) {
      throw new Error("Command description is required for ChatInput commands");
    }
    return {
      name: this.name,
      description: this.description,
      options: this.options,
      command_type: this.command_type,
      ...(this.default_member_permissions !== undefined && {
        default_member_permissions: this.default_member_permissions,
      }),
      dm_permission: this.dm_permission,
    };
  }

  toJSON() {
    return this.build();
  }
}
