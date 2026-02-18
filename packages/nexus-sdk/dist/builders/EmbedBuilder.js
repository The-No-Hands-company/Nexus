"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.EmbedBuilder = void 0;
/** Fluent builder for rich embeds. */
class EmbedBuilder {
    data = {};
    setTitle(title) {
        this.data.title = title;
        return this;
    }
    setDescription(description) {
        this.data.description = description;
        return this;
    }
    setURL(url) {
        this.data.url = url;
        return this;
    }
    /** Hex color number, e.g. `0x7c6af7` or an integer like `8153847`. */
    setColor(color) {
        this.data.color = color;
        return this;
    }
    /** ISO 8601 timestamp string. Pass a `Date` and it will be converted. */
    setTimestamp(timestamp) {
        const ts = timestamp ?? new Date();
        this.data.timestamp =
            typeof ts === "string" ? ts : ts.toISOString();
        return this;
    }
    setFooter(text, iconUrl) {
        this.data.footer = { text, ...(iconUrl && { icon_url: iconUrl }) };
        return this;
    }
    setImage(url) {
        this.data.image = { url };
        return this;
    }
    setThumbnail(url) {
        this.data.thumbnail = { url };
        return this;
    }
    setAuthor(name, url, iconUrl) {
        this.data.author = {
            name,
            ...(url && { url }),
            ...(iconUrl && { icon_url: iconUrl }),
        };
        return this;
    }
    addField(name, value, inline = false) {
        this.data.fields ??= [];
        this.data.fields.push({ name, value, inline });
        return this;
    }
    addFields(...fields) {
        this.data.fields ??= [];
        this.data.fields.push(...fields);
        return this;
    }
    build() {
        return { ...this.data };
    }
    toJSON() {
        return this.build();
    }
}
exports.EmbedBuilder = EmbedBuilder;
//# sourceMappingURL=EmbedBuilder.js.map