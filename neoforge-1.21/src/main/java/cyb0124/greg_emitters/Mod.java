package cyb0124.greg_emitters;

import com.sun.jna.*;
import com.sun.jna.platform.unix.LibCUtil;
import com.sun.jna.platform.win32.*;

import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Map;

@net.neoforged.fml.common.Mod("greg_emitters")
public class Mod {
    // ELF relocation types
    private static final int R_X86_64_RELATIVE = 8;
    private static final int R_AARCH64_RELATIVE = 1027;

    // mmap protection flags
    private static final int PROT_READ = 1;
    private static final int PROT_WRITE = 2;
    private static final int PROT_EXEC = 4;

    // mmap flags
    private static final int MAP_PRIVATE = 0x0002;
    private static final int MAP_ANON_LINUX = 0x0020;
    private static final int MAP_ANON_MAC = 0x1000;

    // Binary header layout
    private static final int HEADER_SIZE = 16;
    private static final int ENTRY_SYSV64_OFFSET = 16;

    public Mod() {
        boolean isX64 = Platform.ARCH.equals("x86-64");
        boolean isAarch64 = Platform.ARCH.equals("aarch64");
        if (!isX64 && !isAarch64)
            throw new UnsupportedOperationException("Unsupported architecture: " + Platform.ARCH);

        String path = isX64 ? "/x64.bin" : "/aarch64.bin";
        byte[] blob;
        try (InputStream is = Mod.class.getResourceAsStream(path)) {
            blob = is.readAllBytes();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }

        // Read header: [rw_start: u32, reloc_start: u32, reloc_end: u32, entry_win64: u32]
        ByteBuffer header = ByteBuffer.wrap(blob, 0, HEADER_SIZE).order(ByteOrder.LITTLE_ENDIAN);
        int rwStart = header.getInt();
        int relocStart = header.getInt();
        int relocEnd = header.getInt();
        int entryWin64 = header.getInt();

        // Allocate RW memory for code + data (excluding relocations)
        int len = relocStart;
        Pointer mem;
        if (Platform.isWindows()) {
            mem = Kernel32.INSTANCE.VirtualAllocEx(WinBase.INVALID_HANDLE_VALUE, null, new BaseTSD.SIZE_T(len), WinNT.MEM_COMMIT, WinNT.PAGE_READWRITE);
        } else if (Platform.isMac()) {
            mem = LibCUtil.mmap(null, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON_MAC, -1, 0);
        } else {
            mem = LibCUtil.mmap(null, len, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON_LINUX, -1, 0);
        }
        mem.write(0, blob, 0, len);

        // Apply ELF relocations (24-byte Rela entries: offset, type, addend)
        long base = Pointer.nativeValue(mem);
        ByteBuffer relocs = ByteBuffer.wrap(blob, relocStart, relocEnd - relocStart).order(ByteOrder.LITTLE_ENDIAN);
        int expectedType = isX64 ? R_X86_64_RELATIVE : R_AARCH64_RELATIVE;
        while (relocs.hasRemaining()) {
            long offset = relocs.getLong();
            long type = relocs.getLong();
            long addend = relocs.getLong();
            if (type != expectedType)
                throw new RuntimeException("Unexpected relocation type: " + type);
            mem.setLong((int) offset, base + addend);
        }

        // Make code section executable
        long execLen = rwStart;
        if (Platform.isWindows()) {
            NativeLibrary.getInstance("kernel32").getFunction("VirtualProtect").invoke(new Object[]{mem, execLen, WinNT.PAGE_EXECUTE_READ, new WinDef.DWORDByReference()});
        } else {
            NativeLibrary.getInstance("c").getFunction("mprotect").invoke(new Object[]{mem, execLen, PROT_READ | PROT_EXEC});
        }

        // Call entry point
        int entryOffset = Platform.isWindows() ? entryWin64 : ENTRY_SYSV64_OFFSET;
        Pointer entry = mem.share(entryOffset);
        Function.getFunction(entry).invoke(Void.class, new Object[]{JNIEnv.CURRENT, this}, Map.of(Library.OPTION_ALLOW_OBJECTS, true));
    }
}
