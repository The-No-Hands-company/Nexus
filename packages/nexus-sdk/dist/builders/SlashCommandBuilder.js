"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SlashCommandBuilder = exports.SlashCommandOptionBuilder = exports.OptionType = void 0;
// ============================================================================
// SlashCommandOptionBuilder
// ============================================================================
/** Option type values that match the server schema. */
exports.OptionType = {
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
};
class SlashCommandOptionBuilder {
    data = {
        name: "",
        description: "",
        option_type: exports.OptionType.String,
        required: false,
        choices: [],
        options: [],
        autocomplete: false,
        min_value: null,
        max_value: null,
    };
    setType(type) {
        this.data.option_type = type;
        return this;
    }
    setName(name) {
        this.data.name = name;
        return this;
    }
    setDescription(description) {
        this.data.description = description;
        return this;
    }
    setRequired(required = true) {
        this.data.required = required;
        return this;
    }
    addChoice(name, value) {
        this.data.choices ??= [];
        this.data.choices.push({ name, value });
        return this;
    }
    addChoices(...choices) {
        this.data.choices ??= [];
        this.data.choices.push(...choices);
        return this;
    }
    setMinValue(min) {
        this.data.min_value = min;
        return this;
    }
    setMaxValue(max) {
        this.data.max_value = max;
        return this;
    }
    setAutocomplete(autocomplete = true) {
        this.data.autocomplete = autocomplete;
        return this;
    }
    addOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder());
        this.data.options ??= [];
        this.data.options.push(opt.build());
        return this;
    }
    build() {
        if (!this.data.name)
            throw new Error("Option name is required");
        if (!this.data.description)
            throw new Error("Option description is required");
        return this.data;
    }
}
exports.SlashCommandOptionBuilder = SlashCommandOptionBuilder;
// ============================================================================
// SlashCommandBuilder
// ============================================================================
/** Fluent builder for creating slash commands. */
class SlashCommandBuilder {
    name = "";
    description = "";
    options = [];
    command_type = 1; // ChatInput
    default_member_permissions;
    dm_permission = true;
    /** Set the command name (1-32 chars, lowercase, no spaces). */
    setName(name) {
        this.name = name.toLowerCase().replace(/\s+/g, "-");
        return this;
    }
    /** Set the command description (1-100 chars). */
    setDescription(description) {
        this.description = description;
        return this;
    }
    /** Set the command type (1=ChatInput, 2=User, 3=Message). */
    setType(type) {
        this.command_type = type;
        return this;
    }
    /** Restrict to users with these permissions (bitfield string). */
    setDefaultMemberPermissions(permissions) {
        this.default_member_permissions = String(permissions);
        return this;
    }
    /** Whether the command is available in DMs (default true). */
    setDMPermission(dmPermission) {
        this.dm_permission = dmPermission;
        return this;
    }
    /** Add a string option. */
    addStringOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.String));
        this.options.push(opt.build());
        return this;
    }
    /** Add an integer option. */
    addIntegerOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.Integer));
        this.options.push(opt.build());
        return this;
    }
    /** Add a boolean option. */
    addBooleanOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.Boolean));
        this.options.push(opt.build());
        return this;
    }
    /** Add a user option. */
    addUserOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.User));
        this.options.push(opt.build());
        return this;
    }
    /** Add a channel option. */
    addChannelOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.Channel));
        this.options.push(opt.build());
        return this;
    }
    /** Add a role option. */
    addRoleOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.Role));
        this.options.push(opt.build());
        return this;
    }
    /** Add a number (float) option. */
    addNumberOption(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.Number));
        this.options.push(opt.build());
        return this;
    }
    /** Add a sub-command. */
    addSubCommand(fn) {
        const opt = fn(new SlashCommandOptionBuilder().setType(exports.OptionType.SubCommand));
        this.options.push(opt.build());
        return this;
    }
    /** Produce the plain object to send to the API. */
    build() {
        if (!this.name)
            throw new Error("Command name is required");
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
exports.SlashCommandBuilder = SlashCommandBuilder;
//# sourceMappingURL=SlashCommandBuilder.js.map