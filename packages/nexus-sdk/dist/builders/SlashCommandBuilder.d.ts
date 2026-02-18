import type { CommandOption, CommandChoice } from "../types/index.js";
/** Option type values that match the server schema. */
export declare const OptionType: {
    readonly SubCommand: 1;
    readonly SubCommandGroup: 2;
    readonly String: 3;
    readonly Integer: 4;
    readonly Boolean: 5;
    readonly User: 6;
    readonly Channel: 7;
    readonly Role: 8;
    readonly Mentionable: 9;
    readonly Number: 10;
    readonly Attachment: 11;
};
export type OptionType = (typeof OptionType)[keyof typeof OptionType];
export declare class SlashCommandOptionBuilder {
    private data;
    setType(type: OptionType): this;
    setName(name: string): this;
    setDescription(description: string): this;
    setRequired(required?: boolean): this;
    addChoice(name: string, value: string | number): this;
    addChoices(...choices: CommandChoice[]): this;
    setMinValue(min: number): this;
    setMaxValue(max: number): this;
    setAutocomplete(autocomplete?: boolean): this;
    addOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    build(): CommandOption;
}
/** Fluent builder for creating slash commands. */
export declare class SlashCommandBuilder {
    private name;
    private description;
    private options;
    private command_type;
    private default_member_permissions;
    private dm_permission;
    /** Set the command name (1-32 chars, lowercase, no spaces). */
    setName(name: string): this;
    /** Set the command description (1-100 chars). */
    setDescription(description: string): this;
    /** Set the command type (1=ChatInput, 2=User, 3=Message). */
    setType(type: 1 | 2 | 3): this;
    /** Restrict to users with these permissions (bitfield string). */
    setDefaultMemberPermissions(permissions: string | number): this;
    /** Whether the command is available in DMs (default true). */
    setDMPermission(dmPermission: boolean): this;
    /** Add a string option. */
    addStringOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add an integer option. */
    addIntegerOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add a boolean option. */
    addBooleanOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add a user option. */
    addUserOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add a channel option. */
    addChannelOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add a role option. */
    addRoleOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add a number (float) option. */
    addNumberOption(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Add a sub-command. */
    addSubCommand(fn: (opt: SlashCommandOptionBuilder) => SlashCommandOptionBuilder): this;
    /** Produce the plain object to send to the API. */
    build(): {
        name: string;
        description: string;
        options: CommandOption[];
        command_type: number;
        default_member_permissions?: string;
        dm_permission: boolean;
    };
    toJSON(): {
        name: string;
        description: string;
        options: CommandOption[];
        command_type: number;
        default_member_permissions?: string;
        dm_permission: boolean;
    };
}
//# sourceMappingURL=SlashCommandBuilder.d.ts.map