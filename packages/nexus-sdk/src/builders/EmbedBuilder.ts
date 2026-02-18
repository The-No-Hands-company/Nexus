import type { Embed } from "../types/index.js";

/** Fluent builder for rich embeds. */
export class EmbedBuilder {
  private data: Embed = {};

  setTitle(title: string): this {
    this.data.title = title;
    return this;
  }

  setDescription(description: string): this {
    this.data.description = description;
    return this;
  }

  setURL(url: string): this {
    this.data.url = url;
    return this;
  }

  /** Hex color number, e.g. `0x7c6af7` or an integer like `8153847`. */
  setColor(color: number): this {
    this.data.color = color;
    return this;
  }

  /** ISO 8601 timestamp string. Pass a `Date` and it will be converted. */
  setTimestamp(timestamp?: Date | string): this {
    const ts = timestamp ?? new Date();
    this.data.timestamp =
      typeof ts === "string" ? ts : ts.toISOString();
    return this;
  }

  setFooter(text: string, iconUrl?: string): this {
    this.data.footer = { text, ...(iconUrl && { icon_url: iconUrl }) };
    return this;
  }

  setImage(url: string): this {
    this.data.image = { url };
    return this;
  }

  setThumbnail(url: string): this {
    this.data.thumbnail = { url };
    return this;
  }

  setAuthor(name: string, url?: string, iconUrl?: string): this {
    this.data.author = {
      name,
      ...(url && { url }),
      ...(iconUrl && { icon_url: iconUrl }),
    };
    return this;
  }

  addField(name: string, value: string, inline = false): this {
    this.data.fields ??= [];
    this.data.fields.push({ name, value, inline });
    return this;
  }

  addFields(
    ...fields: Array<{ name: string; value: string; inline?: boolean }>
  ): this {
    this.data.fields ??= [];
    this.data.fields.push(...fields);
    return this;
  }

  build(): Embed {
    return { ...this.data };
  }

  toJSON(): Embed {
    return this.build();
  }
}
