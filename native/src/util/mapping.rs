use crate::{asm::*, global::GlobalObjs, jvm::*, mapping_base::*, objs, util::BaseExt};
use alloc::{sync::Arc, vec::Vec};
use bstr::B;
use mapping_macros::Functor;

#[derive(Functor)]
pub struct ForgeCN<T> {
    pub cml: T,
    pub naming_domain: T,
    pub fml: T,
    pub fml_java_ctx: T,
    pub evt_bus: T,
    pub reg_keys: T,
    pub reg_evt: T,
    pub forge_reg: T,
    pub non_null_supplier: T,
    pub lazy_opt: T,
    pub cap: T,
    pub cap_provider: T,
    // Client
    pub renderers_evt: T,
    pub atlas_evt: T,
}

impl ForgeCN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = ForgeCN::<&[u8]> {
            cml: b"cpw.mods.modlauncher.Launcher",
            naming_domain: b"cpw.mods.modlauncher.api.INameMappingService$Domain",
            fml: b"net.minecraftforge.fml.loading.FMLLoader",
            fml_java_ctx: b"net.minecraftforge.fml.javafmlmod.FMLJavaModLoadingContext",
            evt_bus: b"net.minecraftforge.eventbus.api.IEventBus",
            reg_keys: b"net.minecraftforge.registries.ForgeRegistries$Keys",
            reg_evt: b"net.minecraftforge.registries.RegisterEvent",
            forge_reg: b"net.minecraftforge.registries.IForgeRegistry",
            non_null_supplier: b"net.minecraftforge.common.util.NonNullSupplier",
            lazy_opt: b"net.minecraftforge.common.util.LazyOptional",
            cap: b"net.minecraftforge.common.capabilities.Capability",
            cap_provider: b"net.minecraftforge.common.capabilities.CapabilityProvider",
            // Client
            renderers_evt: b"net.minecraftforge.client.event.EntityRenderersEvent$RegisterRenderers",
            atlas_evt: b"net.minecraftforge.client.event.TextureStitchEvent",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

pub struct ForgeMN {
    pub get_cap: MSig,
    pub invalidate_caps: MSig,
    pub non_null_supplier_get: MSig,
}

impl ForgeMN {
    pub fn new(cn: &CN<Arc<CSig>>, fcn: &ForgeCN<Arc<CSig>>) -> Self {
        Self {
            get_cap: MSig {
                owner: fcn.cap_provider.clone(),
                name: cs("getCapability"),
                sig: msig([fcn.cap.sig.to_bytes(), cn.dir.sig.to_bytes()], fcn.lazy_opt.sig.to_bytes()),
            },
            invalidate_caps: MSig { owner: fcn.cap_provider.clone(), name: cs("invalidateCaps"), sig: cs("()V") },
            non_null_supplier_get: MSig { owner: fcn.non_null_supplier.clone(), name: cs("get"), sig: cs("()Ljava/lang/Object;") },
        }
    }
}

pub struct ForgeMV {
    pub cml_inst: GlobalRef<'static>,
    pub cml_get_mapper: usize,
    pub naming_domain_f: GlobalRef<'static>,
    pub naming_domain_m: GlobalRef<'static>,
    pub fml_naming_is_srg: bool,
    pub mod_evt_bus: GlobalRef<'static>,
    pub evt_bus_add_listener: usize,
    pub reg_key_blocks: GlobalRef<'static>,
    pub reg_key_tile_types: GlobalRef<'static>,
    pub reg_key_menu_types: GlobalRef<'static>,
    pub reg_evt_key: usize,
    pub reg_evt_forge_reg: usize,
    pub forge_reg_reg: usize,
    pub lazy_opt: GlobalRef<'static>,
    pub lazy_opt_of: usize,
    pub lazy_opt_invalidate: usize,
    pub cap_provider: GlobalRef<'static>,
    pub get_cap: usize,
    pub invalidate_caps: usize,
    pub client: Option<ForgeMVC>,
}

pub struct ForgeMVC {
    pub renderers_evt_reg: usize,
    pub atlas_evt_get_atlas: usize,
}

impl ForgeMV {
    pub fn new(av: &AV<'static>, cn: &CN<Arc<CSig>>, fcn: &ForgeCN<Arc<CSig>>, fmn: &ForgeMN) -> Self {
        let load = |csig: &Arc<CSig>| av.ldr.load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let cml = load(&fcn.cml);
        let naming_domain = load(&fcn.naming_domain);
        let fml = av.ldr.load_class(&av.jv, c"net.minecraftforge.fml.loading.FMLLoader").unwrap().new_global_ref().unwrap();
        let fml_naming = fml.get_static_field_id(c"naming", c"Ljava/lang/String;").unwrap();
        let fml_naming_is_srg = &*fml.get_static_object_field(fml_naming).unwrap().utf_chars().unwrap() == b"srg";
        let fml_java_ctx = load(&fcn.fml_java_ctx);
        let fml_ctx_inst = fml_java_ctx.get_static_method_id(c"get", &msig([], fcn.fml_java_ctx.sig.to_bytes())).unwrap();
        let fml_ctx_inst = fml_java_ctx.call_static_object_method(fml_ctx_inst, &[]).unwrap().unwrap();
        let evt_bus = load(&fcn.evt_bus);
        let mod_evt_bus = fml_java_ctx.get_method_id(c"getModEventBus", &msig([], fcn.evt_bus.sig.to_bytes())).unwrap();
        let mod_evt_bus = fml_ctx_inst.call_object_method(mod_evt_bus, &[]).unwrap().unwrap().new_global_ref().unwrap();
        let reg_keys = load(&fcn.reg_keys);
        let reg_evt = load(&fcn.reg_evt);
        let lazy_opt = load(&fcn.lazy_opt);
        let cap_provider = load(&fcn.cap_provider);
        let dist = fml.static_field_1(c"dist", c"Lnet/minecraftforge/api/distmarker/Dist;");
        let is_client = dist.call_bool_method(dist.get_object_class().get_method_id(c"isClient", c"()Z").unwrap(), &[]).unwrap();
        Self {
            cml_inst: cml.static_field_1(c"INSTANCE", &fcn.cml.sig),
            cml_get_mapper: cml.get_method_id(c"findNameMapping", c"(Ljava/lang/String;)Ljava/util/Optional;").unwrap(),
            naming_domain_f: naming_domain.static_field_1(c"FIELD", &fcn.naming_domain.sig),
            naming_domain_m: naming_domain.static_field_1(c"METHOD", &fcn.naming_domain.sig),
            fml_naming_is_srg,
            mod_evt_bus,
            evt_bus_add_listener: evt_bus.get_method_id(c"addListener", c"(Ljava/util/function/Consumer;)V").unwrap(),
            reg_key_blocks: reg_keys.static_field_1(c"BLOCKS", &cn.resource_key.sig),
            reg_key_tile_types: reg_keys.static_field_1(c"BLOCK_ENTITY_TYPES", &cn.resource_key.sig),
            reg_key_menu_types: reg_keys.static_field_1(c"MENU_TYPES", &cn.resource_key.sig),
            reg_evt_key: reg_evt.get_method_id(c"getRegistryKey", &msig([], cn.resource_key.sig.to_bytes())).unwrap(),
            reg_evt_forge_reg: reg_evt.get_method_id(c"getForgeRegistry", &msig([], fcn.forge_reg.sig.to_bytes())).unwrap(),
            forge_reg_reg: load(&fcn.forge_reg).get_method_id(c"register", c"(Ljava/lang/String;Ljava/lang/Object;)V").unwrap(),
            lazy_opt_of: lazy_opt.get_static_method_id(c"of", &msig([fcn.non_null_supplier.sig.to_bytes()], fcn.lazy_opt.sig.to_bytes())).unwrap(),
            lazy_opt_invalidate: lazy_opt.get_method_id(c"invalidate", c"()V").unwrap(),
            lazy_opt,
            get_cap: fmn.get_cap.get_method_id(&cap_provider).unwrap(),
            invalidate_caps: fmn.invalidate_caps.get_method_id(&cap_provider).unwrap(),
            cap_provider,
            client: is_client.then(|| {
                let renderers_evt = load(&fcn.renderers_evt);
                let renderers_evt_reg = msig([cn.tile_type.sig.to_bytes(), cn.tile_renderer_provider.sig.to_bytes()], b"V");
                let atlas_evt = load(&fcn.atlas_evt);
                ForgeMVC {
                    renderers_evt_reg: renderers_evt.get_method_id(c"registerBlockEntityRenderer", &renderers_evt_reg).unwrap(),
                    atlas_evt_get_atlas: atlas_evt.get_method_id(c"getAtlas", &msig([], cn.atlas.sig.to_bytes())).unwrap(),
                }
            }),
        }
    }
}

#[derive(Functor)]
pub struct CN<T> {
    pub block: T,
    pub block_beh: T,
    pub block_beh_props: T,
    pub block_getter: T,
    pub blocks: T,
    pub level: T,
    pub base_tile_block: T,
    pub tile_block: T,
    pub resource_key: T,
    pub item: T,
    pub item_props: T,
    pub item_stack: T,
    pub item_like: T,
    pub block_item: T,
    pub tile: T,
    pub tile_supplier: T,
    pub tile_type: T,
    pub vec3i: T,
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
    pub block_place_ctx: T,
    pub use_on_ctx: T,
    pub interaction_result: T,
    pub s2c_tile_data: T,
    pub nbt_compound: T,
    pub packet: T,
    pub living_entity: T,
    pub dir: T,
    pub loot_builder: T,
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
            blocks: b"net.minecraft.world.level.block.Blocks",
            level: b"net.minecraft.world.level.Level",
            base_tile_block: b"net.minecraft.world.level.block.BaseEntityBlock",
            tile_block: b"net.minecraft.world.level.block.EntityBlock",
            resource_key: b"net.minecraft.resources.ResourceKey",
            item: b"net.minecraft.world.item.Item",
            item_props: b"net.minecraft.world.item.Item$Properties",
            item_stack: b"net.minecraft.world.item.ItemStack",
            item_like: b"net.minecraft.world.level.ItemLike",
            block_item: b"net.minecraft.world.item.BlockItem",
            tile: b"net.minecraft.world.level.block.entity.BlockEntity",
            tile_supplier: b"net.minecraft.world.level.block.entity.BlockEntityType$BlockEntitySupplier",
            tile_type: b"net.minecraft.world.level.block.entity.BlockEntityType",
            vec3i: b"net.minecraft.core.Vec3i",
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
            block_place_ctx: b"net.minecraft.world.item.context.BlockPlaceContext",
            use_on_ctx: b"net.minecraft.world.item.context.UseOnContext",
            interaction_result: b"net.minecraft.world.InteractionResult",
            s2c_tile_data: b"net.minecraft.network.protocol.game.ClientboundBlockEntityDataPacket",
            nbt_compound: b"net.minecraft.nbt.CompoundTag",
            packet: b"net.minecraft.network.protocol.Packet",
            living_entity: b"net.minecraft.world.entity.LivingEntity",
            dir: b"net.minecraft.core.Direction",
            loot_builder: b"net.minecraft.world.level.storage.loot.LootParams$Builder",
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
    pub block_default_state: T,
    pub block_beh_props_of: T,
    pub block_beh_props_strength: T,
    pub block_beh_props_dyn_shape: T,
    pub block_beh_props_sound: T,
    pub block_beh_get_render_shape: T,
    pub block_beh_get_shape: T,
    pub block_beh_get_drops: T,
    pub block_beh_on_place: T,
    pub block_item_init: T,
    pub block_item_place_block: T,
    pub block_getter_get_block_state: T,
    pub block_getter_get_tile: T,
    pub vec3i_x: T,
    pub vec3i_y: T,
    pub vec3i_z: T,
    pub block_pos_init: T,
    pub block_state_get_block: T,
    pub blocks_fire: T,
    pub tile_supplier_create: T,
    pub tile_type_init: T,
    pub tile_init: T,
    pub tile_load: T,
    pub tile_get_update_tag: T,
    pub tile_get_update_packet: T,
    pub tile_save_additional: T,
    pub tile_level: T,
    pub tile_pos: T,
    pub sound_type_metal: T,
    pub item_get_desc_id: T,
    pub item_stack_init: T,
    pub creative_tab_items_gen_accept: T,
    pub render_shape_tile: T,
    pub resource_loc_init: T,
    pub shapes_create: T,
    pub s2c_tile_data_create: T,
    pub nbt_compound_init: T,
    pub nbt_compound_put_byte_array: T,
    pub nbt_compound_get_byte_array: T,
    pub use_on_ctx_get_level: T,
    pub use_on_ctx_get_clicked_pos: T,
    pub use_on_ctx_get_clicked_face: T,
    pub dir_3d_data: T,
    pub dir_by_3d_data: T,
    pub level_set_block_and_update: T,
    pub level_update_neighbors_for_out_signal: T,
    pub level_is_client: T,
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
    pub fn new(av: &AV, cn: &CN<Arc<CSig>>, fmv: &ForgeMV) -> Self {
        let mut mn = MN {
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
            block_default_state: MSig { owner: cn.block.clone(), name: cs("m_49966_"), sig: msig([], cn.block_state.sig.to_bytes()) },
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
            block_beh_get_drops: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_49635_"),
                sig: msig([cn.block_state.sig.to_bytes(), cn.loot_builder.sig.to_bytes()], b"Ljava/util/List;"),
            },
            block_beh_on_place: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_6807_"),
                sig: msig(
                    [cn.block_state.sig.to_bytes(), cn.level.sig.to_bytes(), cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes(), b"Z"],
                    b"V",
                ),
            },
            block_item_init: MSig {
                owner: cn.block_item.clone(),
                name: cs("<init>"),
                sig: msig([cn.block.sig.to_bytes(), cn.item_props.sig.to_bytes()], b"V"),
            },
            block_item_place_block: MSig {
                owner: cn.block_item.clone(),
                name: cs("m_7429_"),
                sig: msig([cn.block_place_ctx.sig.to_bytes(), cn.block_state.sig.to_bytes()], b"Z"),
            },
            block_getter_get_block_state: MSig {
                owner: cn.block_getter.clone(),
                name: cs("m_8055_"),
                sig: msig([cn.block_pos.sig.to_bytes()], cn.block_state.sig.to_bytes()),
            },
            block_getter_get_tile: MSig {
                owner: cn.block_getter.clone(),
                name: cs("m_7702_"),
                sig: msig([cn.block_pos.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            vec3i_x: MSig { owner: cn.vec3i.clone(), name: cs("f_123285_"), sig: cs("I") },
            vec3i_y: MSig { owner: cn.vec3i.clone(), name: cs("f_123286_"), sig: cs("I") },
            vec3i_z: MSig { owner: cn.vec3i.clone(), name: cs("f_123289_"), sig: cs("I") },
            block_pos_init: MSig { owner: cn.block_pos.clone(), name: cs("<init>"), sig: cs("(III)V") },
            block_state_get_block: MSig { owner: cn.block_state.clone(), name: cs("m_60734_"), sig: msig([], cn.block.sig.to_bytes()) },
            blocks_fire: MSig { owner: cn.blocks.clone(), name: cs("f_50083_"), sig: cn.block.sig.clone() },
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
            tile_save_additional: MSig { owner: cn.tile.clone(), name: cs("m_183515_"), sig: msig([cn.nbt_compound.sig.to_bytes()], b"V") },
            tile_level: MSig { owner: cn.tile.clone(), name: cs("f_58857_"), sig: cn.level.sig.clone() },
            tile_pos: MSig { owner: cn.tile.clone(), name: cs("f_58858_"), sig: cn.block_pos.sig.clone() },
            sound_type_metal: MSig { owner: cn.sound_type.clone(), name: cs("f_56743_"), sig: cn.sound_type.sig.clone() },
            item_get_desc_id: MSig { owner: cn.item.clone(), name: cs("m_5524_"), sig: cs("()Ljava/lang/String;") },
            item_stack_init: MSig {
                owner: cn.item_stack.clone(),
                name: cs("<init>"),
                sig: msig([cn.item_like.sig.to_bytes(), b"I", cn.nbt_compound.sig.to_bytes()], b"V"),
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
            use_on_ctx_get_level: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_43725_"), sig: msig([], cn.level.sig.to_bytes()) },
            use_on_ctx_get_clicked_pos: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_8083_"), sig: msig([], cn.block_pos.sig.to_bytes()) },
            use_on_ctx_get_clicked_face: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_43719_"), sig: msig([], cn.dir.sig.to_bytes()) },
            dir_3d_data: MSig { owner: cn.dir.clone(), name: cs("f_122339_"), sig: cs("I") },
            dir_by_3d_data: MSig {
                owner: cn.dir.clone(),
                name: cs("f_122348_"),
                sig: cs(Vec::from_iter(b"[".iter().chain(cn.dir.sig.as_bytes()).copied())),
            },
            level_set_block_and_update: MSig {
                owner: cn.level.clone(),
                name: cs("m_46597_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], b"Z"),
            },
            level_update_neighbors_for_out_signal: MSig {
                owner: cn.level.clone(),
                name: cs("m_46717_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block.sig.to_bytes()], b"V"),
            },
            level_is_client: MSig { owner: cn.level.clone(), name: cs("f_46443_"), sig: cs("Z") },
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
        };
        if !fmv.fml_naming_is_srg {
            let from = av.ldr.jni.new_utf(c"srg").unwrap();
            let mapper = fmv.cml_inst.call_object_method(fmv.cml_get_mapper, &[from.raw]).unwrap().unwrap().opt_get(&av.jv).unwrap().unwrap();
            mn.apply(|x| {
                let name = mapper.jni.new_utf(&x.name).unwrap();
                let name = mapper.bifunc_apply(&av.jv, if x.is_method() { fmv.naming_domain_m.raw } else { fmv.naming_domain_f.raw }, name.raw);
                x.name = cs(name.unwrap().unwrap().utf_chars().unwrap().to_vec())
            })
        }
        mn
    }
}

pub struct MV {
    pub base_tile_block_init: usize,
    pub block_default_state: usize,
    pub block_beh_props: GlobalRef<'static>,
    pub block_beh_props_of: usize,
    pub block_beh_props_strength: usize,
    pub block_beh_props_dyn_shape: usize,
    pub block_beh_props_sound: usize,
    pub block_item: GlobalRef<'static>,
    pub block_item_init: usize,
    pub block_item_place_block: usize,
    pub block_getter_get_block_state: usize,
    pub block_getter_get_tile: usize,
    pub vec3i_x: usize,
    pub vec3i_y: usize,
    pub vec3i_z: usize,
    pub block_pos: GlobalRef<'static>,
    pub block_pos_init: usize,
    pub block_state_get_block: usize,
    pub blocks_fire: GlobalRef<'static>,
    pub tile_type: GlobalRef<'static>,
    pub tile_type_init: usize,
    pub tile: GlobalRef<'static>,
    pub tile_init: usize,
    pub tile_load: usize,
    pub tile_save_additional: usize,
    pub tile_level: usize,
    pub tile_pos: usize,
    pub sound_type_metal: GlobalRef<'static>,
    pub item: GlobalRef<'static>,
    pub item_get_desc_id: usize,
    pub item_stack: GlobalRef<'static>,
    pub item_stack_init: usize,
    pub render_shape_tile: GlobalRef<'static>,
    pub resource_loc: GlobalRef<'static>,
    pub resource_loc_init: usize,
    pub shapes: GlobalRef<'static>,
    pub shapes_create: usize,
    pub nbt_compound: GlobalRef<'static>,
    pub nbt_compound_init: usize,
    pub nbt_compound_put_byte_array: usize,
    pub nbt_compound_get_byte_array: usize,
    pub use_on_ctx_get_level: usize,
    pub use_on_ctx_get_clicked_pos: usize,
    pub use_on_ctx_get_clicked_face: usize,
    pub dir_3d_data: usize,
    pub dir_by_3d_data: GlobalRef<'static>,
    pub level_set_block_and_update: usize,
    pub level_update_neighbors_for_out_signal: usize,
    pub level_is_client: usize,
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
        let base_tile_block = load(&cn.base_tile_block);
        let block = load(&cn.block);
        let block_beh_props = load(&cn.block_beh_props);
        let block_item = load(&cn.block_item);
        let block_getter = load(&cn.block_getter);
        let vec3i = load(&cn.vec3i);
        let block_pos = load(&cn.block_pos);
        let block_state = load(&cn.block_state);
        let tile_type = load(&cn.tile_type);
        let tile = load(&cn.tile);
        let sound_type = load(&cn.sound_type);
        let item = load(&cn.item);
        let item_stack = load(&cn.item_stack);
        let render_shape = load(&cn.render_shape);
        let resource_loc = load(&cn.resource_loc);
        let shapes = load(&cn.shapes);
        let nbt_compound = load(&cn.nbt_compound);
        let use_on_ctx = load(&cn.use_on_ctx);
        let dir = load(&cn.dir);
        let level = load(&cn.level);
        MV {
            base_tile_block_init: mn.base_tile_block_init.get_method_id(&base_tile_block).unwrap(),
            block_default_state: mn.block_default_state.get_method_id(&block).unwrap(),
            block_beh_props_of: mn.block_beh_props_of.get_static_method_id(&block_beh_props).unwrap(),
            block_beh_props_strength: mn.block_beh_props_strength.get_method_id(&block_beh_props).unwrap(),
            block_beh_props_dyn_shape: mn.block_beh_props_dyn_shape.get_method_id(&block_beh_props).unwrap(),
            block_beh_props_sound: mn.block_beh_props_sound.get_method_id(&block_beh_props).unwrap(),
            block_beh_props,
            block_item_init: mn.block_item_init.get_method_id(&block_item).unwrap(),
            block_item_place_block: mn.block_item_place_block.get_method_id(&block_item).unwrap(),
            block_item,
            block_getter_get_block_state: mn.block_getter_get_block_state.get_method_id(&block_getter).unwrap(),
            block_getter_get_tile: mn.block_getter_get_tile.get_method_id(&block_getter).unwrap(),
            vec3i_x: mn.vec3i_x.get_field_id(&vec3i).unwrap(),
            vec3i_y: mn.vec3i_y.get_field_id(&vec3i).unwrap(),
            vec3i_z: mn.vec3i_z.get_field_id(&vec3i).unwrap(),
            block_pos_init: mn.block_pos_init.get_method_id(&block_pos).unwrap(),
            block_pos,
            block_state_get_block: mn.block_state_get_block.get_method_id(&block_state).unwrap(),
            blocks_fire: load(&cn.blocks).static_field_2(&mn.blocks_fire),
            tile_type_init: mn.tile_type_init.get_method_id(&tile_type).unwrap(),
            tile_type,
            tile_init: mn.tile_init.get_method_id(&tile).unwrap(),
            tile_load: mn.tile_load.get_method_id(&tile).unwrap(),
            tile_save_additional: mn.tile_save_additional.get_method_id(&tile).unwrap(),
            tile_level: mn.tile_level.get_field_id(&tile).unwrap(),
            tile_pos: mn.tile_pos.get_field_id(&tile).unwrap(),
            tile,
            sound_type_metal: sound_type.static_field_2(&mn.sound_type_metal),
            item_get_desc_id: mn.item_get_desc_id.get_method_id(&item).unwrap(),
            item,
            item_stack_init: mn.item_stack_init.get_method_id(&item_stack).unwrap(),
            item_stack,
            render_shape_tile: render_shape.static_field_2(&mn.render_shape_tile),
            resource_loc_init: mn.resource_loc_init.get_method_id(&resource_loc).unwrap(),
            resource_loc,
            shapes_create: mn.shapes_create.get_static_method_id(&shapes).unwrap(),
            shapes,
            nbt_compound_init: mn.nbt_compound_init.get_method_id(&nbt_compound).unwrap(),
            nbt_compound_put_byte_array: mn.nbt_compound_put_byte_array.get_method_id(&nbt_compound).unwrap(),
            nbt_compound_get_byte_array: mn.nbt_compound_get_byte_array.get_method_id(&nbt_compound).unwrap(),
            nbt_compound,
            use_on_ctx_get_level: mn.use_on_ctx_get_level.get_method_id(&use_on_ctx).unwrap(),
            use_on_ctx_get_clicked_pos: mn.use_on_ctx_get_clicked_pos.get_method_id(&use_on_ctx).unwrap(),
            use_on_ctx_get_clicked_face: mn.use_on_ctx_get_clicked_face.get_method_id(&use_on_ctx).unwrap(),
            dir_3d_data: mn.dir_3d_data.get_field_id(&dir).unwrap(),
            dir_by_3d_data: dir.static_field_2(&mn.dir_by_3d_data),
            level_set_block_and_update: mn.level_set_block_and_update.get_method_id(&level).unwrap(),
            level_update_neighbors_for_out_signal: mn.level_update_neighbors_for_out_signal.get_method_id(&level).unwrap(),
            level_is_client: mn.level_is_client.get_field_id(&level).unwrap(),
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
                    atlas_loc_blocks: atlas.static_field_2(&mn.atlas_loc_blocks),
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
    pub caps: T,
    pub dyn_resource_pack: T,
    pub material_block_renderer: T,
    pub energy_container: T,
    pub pipe_block: T,
    pub pipe_node: T,
}

impl GregCN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = GregCN::<&[u8]> {
            reg: b"com.gregtechceu.gtceu.api.registry.registrate.GTRegistrate",
            item_builder: b"com.tterrag.registrate.builders.ItemBuilder",
            non_null_fn: b"com.tterrag.registrate.util.nullness.NonNullFunction",
            creative_tab_items_gen: b"com.gregtechceu.gtceu.common.data.GTCreativeModeTabs$RegistrateDisplayItemsGenerator",
            values: b"com.gregtechceu.gtceu.api.GTValues",
            caps: b"com.gregtechceu.gtceu.api.capability.forge.GTCapability",
            dyn_resource_pack: b"com.gregtechceu.gtceu.data.pack.GTDynamicResourcePack",
            material_block_renderer: b"com.gregtechceu.gtceu.client.renderer.block.MaterialBlockRenderer",
            energy_container: b"com.gregtechceu.gtceu.api.capability.IEnergyContainer",
            pipe_block: b"com.gregtechceu.gtceu.api.block.PipeBlock",
            pipe_node: b"com.gregtechceu.gtceu.api.pipenet.IPipeNode",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

pub struct GregMN {
    pub reg_item: MSig,
    pub can_input_eu_from_side: MSig,
    pub accept_eu: MSig,
    pub change_eu: MSig,
    pub get_eu_stored: MSig,
    pub get_eu_capacity: MSig,
    pub get_input_amps: MSig,
    pub get_input_volts: MSig,
    pub get_input_eu_per_sec: MSig,
    pub pipe_block_get_node: MSig,
    pub pipe_block_can_connect: MSig,
    pub pipe_node_set_connection: MSig,
}

impl GregMN {
    pub fn new(cn: &CN<Arc<CSig>>, gcn: &GregCN<Arc<CSig>>) -> Self {
        GregMN {
            reg_item: MSig {
                owner: gcn.reg.clone(),
                name: cs("item"),
                sig: msig([b"Ljava/lang/String;", gcn.non_null_fn.sig.to_bytes()], gcn.item_builder.sig.to_bytes()),
            },
            can_input_eu_from_side: MSig { owner: gcn.energy_container.clone(), name: cs("inputsEnergy"), sig: msig([cn.dir.sig.to_bytes()], b"Z") },
            accept_eu: MSig {
                owner: gcn.energy_container.clone(),
                name: cs("acceptEnergyFromNetwork"),
                sig: msig([cn.dir.sig.to_bytes(), b"JJ"], b"J"),
            },
            change_eu: MSig { owner: gcn.energy_container.clone(), name: cs("changeEnergy"), sig: cs("(J)J") },
            get_eu_stored: MSig { owner: gcn.energy_container.clone(), name: cs("getEnergyStored"), sig: cs("()J") },
            get_eu_capacity: MSig { owner: gcn.energy_container.clone(), name: cs("getEnergyCapacity"), sig: cs("()J") },
            get_input_amps: MSig { owner: gcn.energy_container.clone(), name: cs("getInputAmperage"), sig: cs("()J") },
            get_input_volts: MSig { owner: gcn.energy_container.clone(), name: cs("getInputVoltage"), sig: cs("()J") },
            get_input_eu_per_sec: MSig { owner: gcn.energy_container.clone(), name: cs("getInputPerSec"), sig: cs("()J") },
            pipe_block_get_node: MSig {
                owner: gcn.pipe_block.clone(),
                name: cs("getPipeTile"),
                sig: msig([cn.block_getter.sig.to_bytes(), cn.block_pos.sig.to_bytes()], gcn.pipe_node.sig.to_bytes()),
            },
            pipe_block_can_connect: MSig {
                owner: gcn.pipe_block.clone(),
                name: cs("canPipeConnectToBlock"),
                sig: msig([gcn.pipe_node.sig.to_bytes(), cn.dir.sig.to_bytes(), cn.tile.sig.to_bytes()], b"Z"),
            },
            pipe_node_set_connection: MSig {
                owner: gcn.pipe_node.clone(),
                name: cs("setConnection"),
                sig: msig([cn.dir.sig.to_bytes(), b"ZZ"], b"V"),
            },
        }
    }
}

pub struct GregMV {
    pub tier_names: GlobalRef<'static>,
    pub tier_volts: GlobalRef<'static>,
    pub dyn_resource_pack_data: GlobalRef<'static>,
    pub energy_container_cap: GlobalRef<'static>,
    pub pipe_block: GlobalRef<'static>,
    pub pipe_block_get_node: usize,
    pub pipe_block_can_connect: usize,
    pub pipe_node_set_connection: usize,
}

impl GregMV {
    pub fn new(jni: &'static JNI) -> Self {
        let GlobalObjs { av, fcn, gcn, gmn, .. } = objs();
        let load = |csig: &Arc<CSig>| av.ldr.with_jni(jni).load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let values = load(&gcn.values);
        let pipe_block = load(&gcn.pipe_block);
        Self {
            tier_names: values.static_field_1(c"VN", c"[Ljava/lang/String;"),
            tier_volts: values.static_field_1(c"V", c"[J"),
            dyn_resource_pack_data: load(&gcn.dyn_resource_pack).static_field_1(c"DATA", c"Ljava/util/concurrent/ConcurrentMap;"),
            energy_container_cap: load(&gcn.caps).static_field_1(c"CAPABILITY_ENERGY_CONTAINER", &fcn.cap.sig),
            pipe_block_get_node: gmn.pipe_block_get_node.get_method_id(&pipe_block).unwrap(),
            pipe_block_can_connect: gmn.pipe_block_can_connect.get_method_id(&pipe_block).unwrap(),
            pipe_node_set_connection: gmn.pipe_node_set_connection.get_method_id(&load(&gcn.pipe_node)).unwrap(),
            pipe_block,
        }
    }
}