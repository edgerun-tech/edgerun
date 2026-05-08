declare module 'google-protobuf' {
  export class Message {
    static [key: string]: any;
    [key: string]: any;
    constructor(data?: any);
    toObject(): any;
    serializeBinary(): Uint8Array;
    static deserializeBinary(bytes: Uint8Array): any;
    static toObject(message: any, options?: any): any;
    static initialize(message: any, data: any[], messageId: number, suggestedPivot: number, repeatedFields?: number[], oneofFields?: number[][]): void;
    static getField(message: any, fieldNumber: number): any;
    static getFieldWithDefault<T>(message: any, fieldNumber: number, defaultValue: T): T;
    static setField(message: any, fieldNumber: number, value: any): void;
    static getWrapperField<T>(message: any, ctor: { new (...args: any[]): T }, fieldNumber: number): T;
    static setWrapperField(message: any, fieldNumber: number, value: any): void;
    static getRepeatedWrapperField<T>(message: any, ctor: { new (...args: any[]): T }, fieldNumber: number): T[];
    static setRepeatedWrapperField(message: any, fieldNumber: number, value: any[]): void;
    static setOneofField(message: any, fieldNumber: number, oneof: number[], value: any): void;
    static setOneofWrapperField(message: any, fieldNumber: number, oneof: number[], value: any): void;
    static computeOneofCase(message: any, oneof: number[]): number;
  }
  
  export class BinaryReader {
    [key: string]: any;
    constructor(bytes?: Uint8Array);
    getFieldNumber(): number;
    nextField(): boolean;
    isEndGroup(): boolean;
    skipField(): void;
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
    readMessage(message: any, reader: BinaryReader | (() => void)): void;
  }
  
  export class BinaryWriter {
    [key: string]: any;
    constructor();
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
    writeMessage(field: number, message: any, writer: BinaryWriter | (() => void)): void;
    getResultBuffer(): Uint8Array;
  }

  export class Map<K = any, V = any> {
    constructor(arr?: any[], valueCtor?: any);
    static deserializeBinary(map: Map<any, any>, reader: BinaryReader, keyReaderFn: Function, valueReaderFn: Function, valueCtor?: any, defaultKey?: any, defaultValue?: any): void;
    toObject(includeInstance?: boolean, valueToObject?: Function): any[];
    serializeBinary(field: number, writer: BinaryWriter, keyWriterFn: Function, valueWriterFn: Function): void;
  }
}
