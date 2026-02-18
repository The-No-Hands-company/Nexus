import type { Embed } from "../types/index.js";
/** Fluent builder for rich embeds. */
export declare class EmbedBuilder {
    private data;
    setTitle(title: string): this;
    setDescription(description: string): this;
    setURL(url: string): this;
    /** Hex color number, e.g. `0x7c6af7` or an integer like `8153847`. */
    setColor(color: number): this;
    /** ISO 8601 timestamp string. Pass a `Date` and it will be converted. */
    setTimestamp(timestamp?: Date | string): this;
    setFooter(text: string, iconUrl?: string): this;
    setImage(url: string): this;
    setThumbnail(url: string): this;
    setAuthor(name: string, url?: string, iconUrl?: string): this;
    addField(name: string, value: string, inline?: boolean): this;
    addFields(...fields: Array<{
        name: string;
        value: string;
        inline?: boolean;
    }>): this;
    build(): Embed;
    toJSON(): Embed;
}
//# sourceMappingURL=EmbedBuilder.d.ts.map