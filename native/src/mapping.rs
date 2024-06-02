use crate::{asm::*, global::GlobalObjs, jvm::*, mapping_base::*, objs};
use alloc::sync::Arc;
use bstr::B;
use mapping_macros::Functor;

pub struct Forge {
    pub cml: GlobalRef<'static>,
    pub cml_get_mapper: usize,
    pub dom_f: GlobalRef<'static>,
    pub dom_m: GlobalRef<'static>,
    pub fml_naming_is_srg: bool,
    pub mod_evt_bus: GlobalRef<'static>,
    pub evt_bus_add_listener: usize,
    pub key_blocks: GlobalRef<'static>,
    pub key_tile_types: GlobalRef<'static>,
    pub reg_evt_sig: CSig,
    pub reg_evt_key: usize,
    pub reg_evt_fg_reg: usize,
    pub fg_reg_reg: usize,
    pub client: Option<ForgeC>,
}

pub struct ForgeC {
    pub renderers_evt_sig: CSig,
    pub renderers_evt_reg: usize,
    pub atlas_evt_sig: CSig,
    pub atlas_evt_get_atlas: usize,
}

impl Forge {
    pub fn new(av: &AV<'static>, cn: &CN<Arc<CSig>>) -> Self {
        let cml_sig = CSig::new(b"cpw.mods.modlauncher.Launcher");
        let cml_cls = av.ldr.load_class(&av.jv, &cml_sig.dot).unwrap();
        let dom_sig = CSig::new(b"cpw.mods.modlauncher.api.INameMappingService$Domain");
        let dom_cls = av.ldr.load_class(&av.jv, &dom_sig.dot).unwrap();
        let fml_cls = av.ldr.load_class(&av.jv, c"net.minecraftforge.fml.loading.FMLLoader").unwrap().new_global_ref().unwrap();
        let fml_naming = fml_cls.get_static_field_id(c"naming", c"Ljava/lang/String;").unwrap();
        let fml_naming_is_srg = &*fml_cls.get_static_object_field(fml_naming).unwrap().utf_chars().unwrap() == b"srg";
        let fml_ctx_sig = CSig::new(b"net.minecraftforge.fml.javafmlmod.FMLJavaModLoadingContext");
        let fml_ctx_cls = av.ldr.load_class(&av.jv, &fml_ctx_sig.dot).unwrap();
        let fml_ctx = fml_ctx_cls.get_static_method_id(c"get", &msig([], fml_ctx_sig.sig.to_bytes())).unwrap();
        let fml_ctx = fml_ctx_cls.call_static_object_method(fml_ctx, &[]).unwrap().unwrap();
        let evt_bus_sig = CSig::new(b"net.minecraftforge.eventbus.api.IEventBus");
        let evt_bus = av.ldr.load_class(&av.jv, &evt_bus_sig.dot).unwrap();
        let mod_evt_bus = fml_ctx_cls.get_method_id(c"getModEventBus", &msig([], evt_bus_sig.sig.to_bytes())).unwrap();
        let mod_evt_bus = fml_ctx.call_object_method(mod_evt_bus, &[]).unwrap().unwrap().new_global_ref().unwrap();
        let reg_keys = av.ldr.load_class(&av.jv, c"net.minecraftforge.registries.ForgeRegistries$Keys").unwrap();
        let key_blocks = reg_keys.get_static_field_id(c"BLOCKS", &cn.resource_key.sig).unwrap();
        let key_tile_types = reg_keys.get_static_field_id(c"BLOCK_ENTITY_TYPES", &cn.resource_key.sig).unwrap();
        let reg_evt_sig = CSig::new(b"net.minecraftforge.registries.RegisterEvent");
        let reg_evt = av.ldr.load_class(&av.jv, &reg_evt_sig.dot).unwrap();
        let fg_reg_sig = CSig::new(b"net.minecraftforge.registries.IForgeRegistry");
        let fg_reg_cls = av.ldr.load_class(&av.jv, &fg_reg_sig.dot).unwrap();
        let dist = fml_cls.get_static_field_id(c"dist", c"Lnet/minecraftforge/api/distmarker/Dist;").unwrap();
        let dist = fml_cls.get_static_object_field(dist).unwrap();
        let is_client = dist.call_bool_method(dist.get_object_class().get_method_id(c"isClient", c"()Z").unwrap(), &[]).unwrap();
        Self {
            cml: cml_cls.get_static_object_field(cml_cls.get_static_field_id(c"INSTANCE", &cml_sig.sig).unwrap()).unwrap().new_global_ref().unwrap(),
            cml_get_mapper: cml_cls.get_method_id(c"findNameMapping", c"(Ljava/lang/String;)Ljava/util/Optional;").unwrap(),
            dom_f: dom_cls.get_static_object_field(dom_cls.get_static_field_id(c"FIELD", &dom_sig.sig).unwrap()).unwrap().new_global_ref().unwrap(),
            dom_m: dom_cls.get_static_object_field(dom_cls.get_static_field_id(c"METHOD", &dom_sig.sig).unwrap()).unwrap().new_global_ref().unwrap(),
            fml_naming_is_srg,
            mod_evt_bus,
            evt_bus_add_listener: evt_bus.get_method_id(c"addListener", c"(Ljava/util/function/Consumer;)V").unwrap(),
            key_blocks: reg_keys.get_static_object_field(key_blocks).unwrap().new_global_ref().unwrap(),
            key_tile_types: reg_keys.get_static_object_field(key_tile_types).unwrap().new_global_ref().unwrap(),
            reg_evt_sig,
            reg_evt_key: reg_evt.get_method_id(c"getRegistryKey", &msig([], cn.resource_key.sig.to_bytes())).unwrap(),
            reg_evt_fg_reg: reg_evt.get_method_id(c"getForgeRegistry", &msig([], fg_reg_sig.sig.to_bytes())).unwrap(),
            fg_reg_reg: fg_reg_cls.get_method_id(c"register", c"(Ljava/lang/String;Ljava/lang/Object;)V").unwrap(),
            client: is_client.then(|| {
                let renderers_evt_sig = CSig::new(b"net.minecraftforge.client.event.EntityRenderersEvent$RegisterRenderers");
                let renderers_evt = av.ldr.load_class(&av.jv, &renderers_evt_sig.dot).unwrap().new_global_ref().unwrap();
                let renderers_evt_reg = msig([cn.tile_type.sig.to_bytes(), cn.tile_renderer_provider.sig.to_bytes()], b"V");
                let atlas_evt_sig = CSig::new(b"net.minecraftforge.client.event.TextureStitchEvent");
                let atlas_evt = av.ldr.load_class(&av.jv, &atlas_evt_sig.dot).unwrap().new_global_ref().unwrap();
                ForgeC {
                    renderers_evt_reg: renderers_evt.get_method_id(c"registerBlockEntityRenderer", &renderers_evt_reg).unwrap(),
                    renderers_evt_sig,
                    atlas_evt_get_atlas: atlas_evt.get_method_id(c"getAtlas", &msig([], cn.atlas.sig.to_bytes())).unwrap(),
                    atlas_evt_sig,
                }
            }),
        }
    }

    pub fn map_mn(&self, av: &AV<'static>, mn: &mut MN<MSig>) {
        let from = av.ldr.jni.new_utf(c"srg").unwrap();
        let mapper = self.cml.call_object_method(self.cml_get_mapper, &[from.raw]).unwrap().unwrap().opt_get(&av.jv).unwrap().unwrap();
        mn.apply(|x| {
            let name = mapper.jni.new_utf(&x.name).unwrap();
            let name = mapper.bifunc_apply(&av.jv, if x.is_method() { self.dom_m.raw } else { self.dom_f.raw }, name.raw);
            x.name = cs(name.unwrap().unwrap().utf_chars().unwrap().to_vec())
        })
    }
}

#[derive(Functor)]
pub struct CN<T> {
    pub block: T,
    pub block_beh: T,
    pub block_beh_props: T,
    pub block_getter: T,
    pub level: T,
    pub base_tile_block: T,
    pub tile_block: T,
    pub resource_key: T,
    pub item: T,
    pub item_props: T,
    pub item_stack: T,
    pub block_item: T,
    pub tile: T,
    pub tile_supplier: T,
    pub tile_type: T,
    pub block_pos: T,
    pub block_state: T,
    pub dfu_type: T,
    pub sound_type: T,
    pub creative_tab_items_gen: T,
    pub creative_tab_params: T,
    pub creative_tab_output: T,
    pub render_shape: T,
    pub resource_loc: T,
    pub voxel_shape: T,
    pub shapes: T,
    pub collision_ctx: T,
    pub use_on_ctx: T,
    pub interaction_result: T,
    pub s2c_tile_data: T,
    pub nbt_compound: T,
    pub packet: T,
    pub living_entity: T,
    pub dir: T,
    // Client
    pub tile_renderer: T,
    pub tile_renderer_provider: T,
    pub tile_renderer_provider_ctx: T,
    pub pose_stack: T,
    pub pose: T,
    pub matrix4f: T,
    pub matrix4fc: T,
    pub atlas: T,
    pub sprite: T,
    pub sheets: T,
    pub buffer_source: T,
    pub vertex_consumer: T,
    pub render_type: T,
}

impl CN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = CN::<&[u8]> {
            block: b"net.minecraft.world.level.block.Block",
            block_beh: b"net.minecraft.world.level.block.state.BlockBehaviour",
            block_beh_props: b"net.minecraft.world.level.block.state.BlockBehaviour$Properties",
            block_getter: b"net.minecraft.world.level.BlockGetter",
            level: b"net.minecraft.world.level.Level",
            base_tile_block: b"net.minecraft.world.level.block.BaseEntityBlock",
            tile_block: b"net.minecraft.world.level.block.EntityBlock",
            resource_key: b"net.minecraft.resources.ResourceKey",
            item: b"net.minecraft.world.item.Item",
            item_props: b"net.minecraft.world.item.Item$Properties",
            item_stack: b"net.minecraft.world.item.ItemStack",
            block_item: b"net.minecraft.world.item.BlockItem",
            tile: b"net.minecraft.world.level.block.entity.BlockEntity",
            tile_supplier: b"net.minecraft.world.level.block.entity.BlockEntityType$BlockEntitySupplier",
            tile_type: b"net.minecraft.world.level.block.entity.BlockEntityType",
            block_pos: b"net.minecraft.core.BlockPos",
            block_state: b"net.minecraft.world.level.block.state.BlockState",
            dfu_type: b"com.mojang.datafixers.types.Type",
            sound_type: b"net.minecraft.world.level.block.SoundType",
            voxel_shape: b"net.minecraft.world.phys.shapes.VoxelShape",
            shapes: b"net.minecraft.world.phys.shapes.Shapes",
            creative_tab_items_gen: b"net.minecraft.world.item.CreativeModeTab$DisplayItemsGenerator",
            creative_tab_params: b"net.minecraft.world.item.CreativeModeTab$ItemDisplayParameters",
            creative_tab_output: b"net.minecraft.world.item.CreativeModeTab$Output",
            render_shape: b"net.minecraft.world.level.block.RenderShape",
            resource_loc: b"net.minecraft.resources.ResourceLocation",
            collision_ctx: b"net.minecraft.world.phys.shapes.CollisionContext",
            use_on_ctx: b"net.minecraft.world.item.context.UseOnContext",
            interaction_result: b"net.minecraft.world.InteractionResult",
            s2c_tile_data: b"net.minecraft.network.protocol.game.ClientboundBlockEntityDataPacket",
            nbt_compound: b"net.minecraft.nbt.CompoundTag",
            packet: b"net.minecraft.network.protocol.Packet",
            living_entity: b"net.minecraft.world.entity.LivingEntity",
            dir: b"net.minecraft.core.Direction",
            // Client
            tile_renderer: b"net.minecraft.client.renderer.blockentity.BlockEntityRenderer",
            tile_renderer_provider: b"net.minecraft.client.renderer.blockentity.BlockEntityRendererProvider",
            tile_renderer_provider_ctx: b"net.minecraft.client.renderer.blockentity.BlockEntityRendererProvider$Context",
            pose_stack: b"com.mojang.blaze3d.vertex.PoseStack",
            pose: b"com.mojang.blaze3d.vertex.PoseStack$Pose",
            matrix4f: b"org.joml.Matrix4f",
            matrix4fc: b"org.joml.Matrix4fc",
            atlas: b"net.minecraft.client.renderer.texture.TextureAtlas",
            sprite: b"net.minecraft.client.renderer.texture.TextureAtlasSprite",
            sheets: b"net.minecraft.client.renderer.Sheets",
            buffer_source: b"net.minecraft.client.renderer.MultiBufferSource",
            vertex_consumer: b"com.mojang.blaze3d.vertex.VertexConsumer",
            render_type: b"net.minecraft.client.renderer.RenderType",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

#[derive(Functor)]
pub struct MN<T> {
    pub base_tile_block_init: T,
    pub tile_block_new_tile: T,
    pub block_set_placed_by: T,
    pub block_beh_props_of: T,
    pub block_beh_props_strength: T,
    pub block_beh_props_dyn_shape: T,
    pub block_beh_props_sound: T,
    pub block_beh_get_render_shape: T,
    pub block_beh_get_shape: T,
    pub block_item_init: T,
    pub block_getter_get_tile: T,
    pub tile_supplier_create: T,
    pub tile_type_init: T,
    pub tile_init: T,
    pub tile_load: T,
    pub tile_get_update_tag: T,
    pub tile_get_update_packet: T,
    pub sound_type_metal: T,
    pub item_get_desc_id: T,
    pub item_use_on: T,
    pub creative_tab_items_gen_accept: T,
    pub render_shape_tile: T,
    pub resource_loc_init: T,
    pub shapes_create: T,
    pub s2c_tile_data_create: T,
    pub nbt_compound_init: T,
    pub nbt_compound_put_byte_array: T,
    pub nbt_compound_get_byte_array: T,
    pub use_on_ctx_get_clicked_face: T,
    pub dir_get_3d_value: T,
    // Client
    pub tile_renderer_render: T,
    pub tile_renderer_provider_create: T,
    pub pose_pose: T,
    pub pose_stack_last: T,
    pub matrix4fc_read: T,
    pub atlas_loc: T,
    pub atlas_loc_blocks: T,
    pub atlas_get_sprite: T,
    pub sprite_u0: T,
    pub sprite_v0: T,
    pub sprite_u1: T,
    pub sprite_v1: T,
    pub sheets_solid: T,
    pub buffer_source_get_buffer: T,
    pub vertex_consumer_vertex: T,
}

impl MN<MSig> {
    pub fn new(cn: &CN<Arc<CSig>>) -> Self {
        MN {
            base_tile_block_init: MSig {
                owner: cn.base_tile_block.clone(),
                name: cs("<init>"),
                sig: msig([cn.block_beh_props.sig.to_bytes()], b"V"),
            },
            tile_block_new_tile: MSig {
                owner: cn.tile_block.clone(),
                name: cs("m_142194_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            block_set_placed_by: MSig {
                owner: cn.block.clone(),
                name: cs("m_6402_"),
                sig: msig(
                    [
                        cn.level.sig.to_bytes(),
                        cn.block_pos.sig.to_bytes(),
                        cn.block_state.sig.to_bytes(),
                        cn.living_entity.sig.to_bytes(),
                        cn.item_stack.sig.to_bytes(),
                    ],
                    b"V",
                ),
            },
            block_beh_props_of: MSig { owner: cn.block_beh_props.clone(), name: cs("m_284310_"), sig: msig([], cn.block_beh_props.sig.to_bytes()) },
            block_beh_props_strength: MSig {
                owner: cn.block_beh_props.clone(),
                name: cs("m_60913_"),
                sig: msig([B("FF")], cn.block_beh_props.sig.to_bytes()),
            },
            block_beh_props_dyn_shape: MSig {
                owner: cn.block_beh_props.clone(),
                name: cs("m_60988_"),
                sig: msig([], cn.block_beh_props.sig.to_bytes()),
            },
            block_beh_props_sound: MSig {
                owner: cn.block_beh_props.clone(),
                name: cs("m_60918_"),
                sig: msig([cn.sound_type.sig.to_bytes()], cn.block_beh_props.sig.to_bytes()),
            },
            block_beh_get_render_shape: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_7514_"),
                sig: msig([cn.block_state.sig.to_bytes()], cn.render_shape.sig.to_bytes()),
            },
            block_beh_get_shape: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_5940_"),
                sig: msig(
                    [cn.block_state.sig.to_bytes(), cn.block_getter.sig.to_bytes(), cn.block_pos.sig.to_bytes(), cn.collision_ctx.sig.to_bytes()],
                    cn.voxel_shape.sig.to_bytes(),
                ),
            },
            block_item_init: MSig {
                owner: cn.block_item.clone(),
                name: cs("<init>"),
                sig: msig([cn.block.sig.to_bytes(), cn.item_props.sig.to_bytes()], b"V"),
            },
            block_getter_get_tile: MSig {
                owner: cn.block_getter.clone(),
                name: cs("m_7702_"),
                sig: msig([cn.block_pos.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            tile_supplier_create: MSig {
                owner: cn.tile_supplier.clone(),
                name: cs("m_155267_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            tile_type_init: MSig {
                owner: cn.tile_type.clone(),
                name: cs("<init>"),
                sig: msig([cn.tile_supplier.sig.to_bytes(), b"Ljava/util/Set;", cn.dfu_type.sig.to_bytes()], b"V"),
            },
            tile_init: MSig {
                owner: cn.tile.clone(),
                name: cs("<init>"),
                sig: msig([cn.tile_type.sig.to_bytes(), cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], b"V"),
            },
            tile_load: MSig { owner: cn.tile.clone(), name: cs("m_142466_"), sig: msig([cn.nbt_compound.sig.to_bytes()], b"V") },
            tile_get_update_tag: MSig { owner: cn.tile.clone(), name: cs("m_5995_"), sig: msig([], cn.nbt_compound.sig.to_bytes()) },
            tile_get_update_packet: MSig { owner: cn.tile.clone(), name: cs("m_58483_"), sig: msig([], cn.packet.sig.to_bytes()) },
            sound_type_metal: MSig { owner: cn.sound_type.clone(), name: cs("f_56743_"), sig: cn.sound_type.sig.clone() },
            item_get_desc_id: MSig { owner: cn.item.clone(), name: cs("m_5524_"), sig: cs("()Ljava/lang/String;") },
            item_use_on: MSig {
                owner: cn.item.clone(),
                name: cs("m_6225_"),
                sig: msig([cn.use_on_ctx.sig.to_bytes()], cn.interaction_result.sig.to_bytes()),
            },
            creative_tab_items_gen_accept: MSig {
                owner: cn.creative_tab_items_gen.clone(),
                name: cs("m_257865_"),
                sig: msig([cn.creative_tab_params.sig.to_bytes(), cn.creative_tab_output.sig.to_bytes()], b"V"),
            },
            render_shape_tile: MSig { owner: cn.render_shape.clone(), name: cs("ENTITYBLOCK_ANIMATED"), sig: cn.render_shape.sig.clone() },
            resource_loc_init: MSig { owner: cn.resource_loc.clone(), name: cs("<init>"), sig: cs("(Ljava/lang/String;Ljava/lang/String;)V") },
            shapes_create: MSig { owner: cn.shapes.clone(), name: cs("m_166049_"), sig: msig([B("DDDDDD")], cn.voxel_shape.sig.to_bytes()) },
            s2c_tile_data_create: MSig {
                owner: cn.s2c_tile_data.clone(),
                name: cs("m_195640_"),
                sig: msig([cn.tile.sig.to_bytes()], cn.s2c_tile_data.sig.to_bytes()),
            },
            nbt_compound_init: MSig { owner: cn.nbt_compound.clone(), name: cs("<init>"), sig: cs("()V") },
            nbt_compound_put_byte_array: MSig { owner: cn.nbt_compound.clone(), name: cs("m_128382_"), sig: cs("(Ljava/lang/String;[B)V") },
            nbt_compound_get_byte_array: MSig { owner: cn.nbt_compound.clone(), name: cs("m_128463_"), sig: cs("(Ljava/lang/String;)[B") },
            use_on_ctx_get_clicked_face: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_43719_"), sig: msig([], cn.dir.sig.to_bytes()) },
            dir_get_3d_value: MSig { owner: cn.dir.clone(), name: cs("m_122411_"), sig: cs("()I") },
            // Client
            tile_renderer_render: MSig {
                owner: cn.tile_renderer.clone(),
                name: cs("m_6922_"),
                sig: msig([cn.tile.sig.to_bytes(), b"F", cn.pose_stack.sig.to_bytes(), cn.buffer_source.sig.to_bytes(), b"II"], b"V"),
            },
            tile_renderer_provider_create: MSig {
                owner: cn.tile_renderer_provider.clone(),
                name: cs("m_173570_"),
                sig: msig([cn.tile_renderer_provider_ctx.sig.to_bytes()], cn.tile_renderer.sig.to_bytes()),
            },
            pose_pose: MSig { owner: cn.pose.clone(), name: cs("f_85852_"), sig: cn.matrix4f.sig.clone() },
            pose_stack_last: MSig { owner: cn.pose_stack.clone(), name: cs("m_85850_"), sig: msig([], cn.pose.sig.to_bytes()) },
            matrix4fc_read: MSig { owner: cn.matrix4fc.clone(), name: cs("getToAddress"), sig: msig([B("J")], cn.matrix4fc.sig.to_bytes()) },
            atlas_loc: MSig { owner: cn.atlas.clone(), name: cs("m_118330_"), sig: msig([], cn.resource_loc.sig.to_bytes()) },
            atlas_loc_blocks: MSig { owner: cn.atlas.clone(), name: cs("f_118259_"), sig: cn.resource_loc.sig.clone() },
            atlas_get_sprite: MSig {
                owner: cn.atlas.clone(),
                name: cs("m_118316_"),
                sig: msig([cn.resource_loc.sig.to_bytes()], cn.sprite.sig.to_bytes()),
            },
            sprite_u0: MSig { owner: cn.sprite.clone(), name: cs("f_118351_"), sig: cs("F") },
            sprite_v0: MSig { owner: cn.sprite.clone(), name: cs("f_118353_"), sig: cs("F") },
            sprite_u1: MSig { owner: cn.sprite.clone(), name: cs("f_118352_"), sig: cs("F") },
            sprite_v1: MSig { owner: cn.sprite.clone(), name: cs("f_118354_"), sig: cs("F") },
            sheets_solid: MSig { owner: cn.sheets.clone(), name: cs("m_110789_"), sig: msig([], cn.render_type.sig.to_bytes()) },
            buffer_source_get_buffer: MSig {
                owner: cn.buffer_source.clone(),
                name: cs("m_6299_"),
                sig: msig([cn.render_type.sig.to_bytes()], cn.vertex_consumer.sig.to_bytes()),
            },
            vertex_consumer_vertex: MSig { owner: cn.vertex_consumer.clone(), name: cs("m_5954_"), sig: cs("(FFFFFFFFFIIFFF)V") },
        }
    }
}

pub struct MV {
    pub base_tile_block_init: usize,
    pub block_beh_props: GlobalRef<'static>,
    pub block_beh_props_of: usize,
    pub block_beh_props_strength: usize,
    pub block_beh_props_dyn_shape: usize,
    pub block_beh_props_sound: usize,
    pub block_item: GlobalRef<'static>,
    pub block_item_init: usize,
    pub block_getter_get_tile: usize,
    pub tile_type: GlobalRef<'static>,
    pub tile_type_init: usize,
    pub tile: GlobalRef<'static>,
    pub tile_init: usize,
    pub tile_load: usize,
    pub sound_type_metal: GlobalRef<'static>,
    pub item: GlobalRef<'static>,
    pub item_get_desc_id: usize,
    pub block_item_use_on: usize,
    pub render_shape_tile: GlobalRef<'static>,
    pub resource_loc: GlobalRef<'static>,
    pub resource_loc_init: usize,
    pub shapes: GlobalRef<'static>,
    pub shapes_create: usize,
    pub nbt_compound: GlobalRef<'static>,
    pub nbt_compound_init: usize,
    pub nbt_compound_put_byte_array: usize,
    pub nbt_compound_get_byte_array: usize,
    pub use_on_ctx_get_clicked_face: usize,
    pub dir_get_3d_value: usize,
    pub client: Option<MVC>,
}

pub struct MVC {
    pub pose_pose: usize,
    pub pose_stack_last: usize,
    pub matrix4fc_read: usize,
    pub atlas_loc: usize,
    pub atlas_loc_blocks: GlobalRef<'static>,
    pub atlas_get_sprite: usize,
    pub sprite_u0: usize,
    pub sprite_v0: usize,
    pub sprite_u1: usize,
    pub sprite_v1: usize,
    pub buffer_source_get_buffer: usize,
    pub vertex_consumer_vertex: usize,
}

impl MV {
    pub fn new(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, is_client: bool) -> MV {
        let load = |csig: &Arc<CSig>| av.ldr.load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let static_field =
            |cv: &GlobalRef<'static>, mn: &MSig| cv.get_static_object_field(mn.get_static_field_id(cv).unwrap()).unwrap().new_global_ref().unwrap();
        let base_tile_block = load(&cn.base_tile_block);
        let block_beh_props = load(&cn.block_beh_props);
        let block_item = load(&cn.block_item);
        let block_getter = load(&cn.block_getter);
        let tile_type = load(&cn.tile_type);
        let tile = load(&cn.tile);
        let sound_type = load(&cn.sound_type);
        let item = load(&cn.item);
        let render_shape = load(&cn.render_shape);
        let resource_loc = load(&cn.resource_loc);
        let shapes = load(&cn.shapes);
        let nbt_compound = load(&cn.nbt_compound);
        let use_on_ctx = load(&cn.use_on_ctx);
        let dir = load(&cn.dir);
        MV {
            base_tile_block_init: mn.base_tile_block_init.get_method_id(&base_tile_block).unwrap(),
            block_beh_props_of: mn.block_beh_props_of.get_static_method_id(&block_beh_props).unwrap(),
            block_beh_props_strength: mn.block_beh_props_strength.get_method_id(&block_beh_props).unwrap(),
            block_beh_props_dyn_shape: mn.block_beh_props_dyn_shape.get_method_id(&block_beh_props).unwrap(),
            block_beh_props_sound: mn.block_beh_props_sound.get_method_id(&block_beh_props).unwrap(),
            block_beh_props,
            block_item_init: mn.block_item_init.get_method_id(&block_item).unwrap(),
            block_item_use_on: mn.item_use_on.get_method_id(&block_item).unwrap(),
            block_item,
            block_getter_get_tile: mn.block_getter_get_tile.get_method_id(&block_getter).unwrap(),
            tile_type_init: mn.tile_type_init.get_method_id(&tile_type).unwrap(),
            tile_type,
            tile_init: mn.tile_init.get_method_id(&tile).unwrap(),
            tile_load: mn.tile_load.get_method_id(&tile).unwrap(),
            tile,
            sound_type_metal: static_field(&sound_type, &mn.sound_type_metal),
            item_get_desc_id: mn.item_get_desc_id.get_method_id(&item).unwrap(),
            item,
            render_shape_tile: static_field(&render_shape, &mn.render_shape_tile),
            resource_loc_init: mn.resource_loc_init.get_method_id(&resource_loc).unwrap(),
            resource_loc,
            shapes_create: mn.shapes_create.get_static_method_id(&shapes).unwrap(),
            shapes,
            nbt_compound_init: mn.nbt_compound_init.get_method_id(&nbt_compound).unwrap(),
            nbt_compound_put_byte_array: mn.nbt_compound_put_byte_array.get_method_id(&nbt_compound).unwrap(),
            nbt_compound_get_byte_array: mn.nbt_compound_get_byte_array.get_method_id(&nbt_compound).unwrap(),
            nbt_compound,
            use_on_ctx_get_clicked_face: mn.use_on_ctx_get_clicked_face.get_method_id(&use_on_ctx).unwrap(),
            dir_get_3d_value: mn.dir_get_3d_value.get_method_id(&dir).unwrap(),
            client: is_client.then(|| {
                let pose = load(&cn.pose);
                let pose_stack = load(&cn.pose_stack);
                let matrix4fc = load(&cn.matrix4fc);
                let atlas = load(&cn.atlas);
                let sprite = load(&cn.sprite);
                let buffer_source = load(&cn.buffer_source);
                let vertex_consumer = load(&cn.vertex_consumer);
                MVC {
                    pose_pose: mn.pose_pose.get_field_id(&pose).unwrap(),
                    pose_stack_last: mn.pose_stack_last.get_method_id(&pose_stack).unwrap(),
                    matrix4fc_read: mn.matrix4fc_read.get_method_id(&matrix4fc).unwrap(),
                    atlas_loc: mn.atlas_loc.get_method_id(&atlas).unwrap(),
                    atlas_loc_blocks: static_field(&atlas, &mn.atlas_loc_blocks),
                    atlas_get_sprite: mn.atlas_get_sprite.get_method_id(&atlas).unwrap(),
                    sprite_u0: mn.sprite_u0.get_field_id(&sprite).unwrap(),
                    sprite_v0: mn.sprite_v0.get_field_id(&sprite).unwrap(),
                    sprite_u1: mn.sprite_u1.get_field_id(&sprite).unwrap(),
                    sprite_v1: mn.sprite_v1.get_field_id(&sprite).unwrap(),
                    buffer_source_get_buffer: mn.buffer_source_get_buffer.get_method_id(&buffer_source).unwrap(),
                    vertex_consumer_vertex: mn.vertex_consumer_vertex.get_method_id(&vertex_consumer).unwrap(),
                }
            }),
        }
    }
}

#[derive(Functor)]
pub struct GregCN<T> {
    pub reg: T,
    pub item_builder: T,
    pub non_null_fn: T,
    pub creative_tab_items_gen: T,
    pub values: T,
}

impl GregCN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = GregCN::<&[u8]> {
            reg: b"com.gregtechceu.gtceu.api.registry.registrate.GTRegistrate",
            item_builder: b"com.tterrag.registrate.builders.ItemBuilder",
            non_null_fn: b"com.tterrag.registrate.util.nullness.NonNullFunction",
            creative_tab_items_gen: b"com.gregtechceu.gtceu.common.data.GTCreativeModeTabs$RegistrateDisplayItemsGenerator",
            values: b"com.gregtechceu.gtceu.api.GTValues",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

pub struct GregMN {
    pub reg_item: MSig,
}

impl GregMN {
    pub fn new(gcn: &GregCN<Arc<CSig>>) -> Self {
        GregMN {
            reg_item: MSig {
                owner: gcn.reg.clone(),
                name: cs("item"),
                sig: msig([b"Ljava/lang/String;", gcn.non_null_fn.sig.to_bytes()], gcn.item_builder.sig.to_bytes()),
            },
        }
    }
}

pub struct GregMV {
    pub tier_names: GlobalRef<'static>,
    pub tier_volts: GlobalRef<'static>,
}

impl GregMV {
    pub fn new(jni: &'static JNI) -> Self {
        let GlobalObjs { av, gcn, .. } = objs();
        let load = |csig: &Arc<CSig>| av.ldr.with_jni(jni).load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let values = load(&gcn.values);
        let tier_names = values.get_static_field_id(c"VN", c"[Ljava/lang/String;").unwrap();
        let tier_volts = values.get_static_field_id(c"V", c"[J").unwrap();
        Self {
            tier_names: values.get_static_object_field(tier_names).unwrap().new_global_ref().unwrap(),
            tier_volts: values.get_static_object_field(tier_volts).unwrap().new_global_ref().unwrap(),
        }
    }
}
