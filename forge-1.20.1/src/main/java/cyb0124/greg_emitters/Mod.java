package cyb0124.greg_emitters;

import com.sun.jna.*;
import com.sun.jna.platform.linux.Mman;
import com.sun.jna.platform.unix.LibCUtil;
import com.sun.jna.platform.win32.*;

import java.io.IOException;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.util.Map;

@net.minecraftforge.fml.common.Mod("greg_emitters")
public class Mod {
    public Mod() {
        String path;
        if (Platform.ARCH.equals("x86-64")) path = "/x64.bin";
        else if (Platform.ARCH.equals("aarch64")) path = "/aarch64.bin";
        else throw new UnsupportedOperationException("Unsupported architecture: " + Platform.ARCH);
        byte[] blob;
        try (InputStream is = Mod.class.getResourceAsStream(path)) {
            blob = is.readAllBytes();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
        ByteBuffer buf = ByteBuffer.wrap(blob, blob.length - 4, 4);
        int len = buf.getInt();
        Pointer mem;
        if (Platform.isWindows()) {
            mem = Kernel32.INSTANCE.VirtualAllocEx(WinBase.INVALID_HANDLE_VALUE, null, new BaseTSD.SIZE_T(len), WinNT.MEM_COMMIT, WinNT.PAGE_READWRITE);
        } else if (Platform.isMac()) {
            mem = LibCUtil.mmap(null, len, /* PROT_READ | PROT_WRITE */ 3, /* MAP_PRIVATE | MAP_ANON */ 0x1002, -1, 0);
        } else {
            mem = LibCUtil.mmap(null, len, /* PROT_READ | PROT_WRITE */ 3, /* MAP_PRIVATE | MAP_ANON */ 0x0022, -1, 0);
        }
        mem.write(0, blob, 0, len);
        buf.position(len);
        while (buf.remaining() > 16) {
            mem.setLong(buf.getInt(), Pointer.nativeValue(mem) + buf.getInt());
        }
        long execLen = (long) buf.getInt() * 4096;
        long allocFns = buf.getInt();
        NativeLibrary c = NativeLibrary.getInstance(Platform.isWindows() ? "msvcrt" : "c");
        mem.setPointer(allocFns, c.getFunction("free"));
        mem.setPointer(allocFns + 8, c.getFunction("malloc"));
        mem.setPointer(allocFns + 16, c.getFunction("realloc"));
        if (Platform.isWindows()) {
            NativeLibrary.getInstance("kernel32").getFunction("VirtualProtect").invoke(new Object[]{mem, execLen, WinNT.PAGE_EXECUTE_READ, new WinDef.DWORDByReference()});
            mem = mem.share(buf.getInt());
        } else {
            c.getFunction("mprotect").invoke(new Object[]{mem, execLen, /* PROT_READ | PROT_EXEC */ 5});
        }
        Function.getFunction(mem).invoke(Void.class, new Object[]{JNIEnv.CURRENT, this}, Map.of(Library.OPTION_ALLOW_OBJECTS, true));
    }
}
