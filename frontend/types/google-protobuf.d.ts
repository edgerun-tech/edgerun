declare module 'google-protobuf' {
  export class Message {
    constructor(data?: any);
    toObject(): any;
    serializeBinary(): Uint8Array;
    static deserializeBinary(bytes: Uint8Array): any;
    static toObject(message: any, options?: any): any;
  }
  
  export class BinaryReader {
    getFieldNumber(): number;
    readString(): string;
    readUint64(): number;
    readInt64(): number;
    readInt32(): number;
    readUint32(): number;
    readBool(): boolean;
    readBytes(): Uint8Array;
    readDouble(): number;
    readFloat(): number;
    readEnum(): number;
    readMessage(message: Message, reader: BinaryReader): void;
  }
  
  export class BinaryWriter {
    writeString(field: number, value: string): void;
    writeUint64(field: number, value: number): void;
    writeInt64(field: number, value: number): void;
    writeInt32(field: number, value: number): void;
    writeUint32(field: number, value: number): void;
    writeBool(field: number, value: boolean): void;
    writeBytes(field: number, value: Uint8Array): void;
    writeDouble(field: number, value: number): void;
    writeFloat(field: number, value: number): void;
    writeEnum(field: number, value: number): void;
    writeMessage(field: number, message: Message, writer: BinaryWriter): void;
  }
}
